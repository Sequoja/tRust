use syntax::ast::DUMMY_NODE_ID;
use syntax::source_map::{symbol::Symbol, DUMMY_SP};
use syntax::{ast, ptr, source_map, ThinVec};

use crate::instfinder::InstPoint;
use instrument::StaticData;

// Constants
const NAME_OF_LOCAL_THREAD_HANDLE_VAR: &str = "instrumentation_local_join_handle";
const NAME_OF_RETURN_VAR: &str = "instrumentation_return_value";
const NAME_OF_INTERMEDIATE_VAR: &str = "instrumentation_intermediate_var_";
const NAME_OF_ARGUMENT_VAR: &str = "instrumentation_argument_var_";
const NAME_OF_INST_CRATE: &str = "instrument";
const NAME_OF_GLOBAL_INIT_FN: &str = "global_init";
const NAME_OF_LOCAL_INIT_FN: &str = "local_init";
const NAME_OF_LOCAL_CLEAN_UP_FN: &str = "clean_up";
const NAME_OF_INSTRUMENT_CALL_FN: &str = "instrument";
const NAME_OF_STATICDATA_STRUCT: &str = "StaticData";
const NAME_OF_STATICDATA_NEW_FN: &str = "new";

const DESCRIPTION_GLOBAL_BEGIN: &str = "GLOBAL_BEGIN";
const DESCRIPTION_GLOBAL_END: &str = "GLOBAL_END";
const DESCRIPTION_LOCAL_BEGIN: &str = "LOCAL_BEGIN";
const DESCRIPTION_LOCAL_END: &str = "LOCAL_END";
const DESCRIPTION_INST_CALL_BEGIN: &str = "BEGIN";
const DESCRIPTION_INST_CALL_END: &str = "END";

// Insert functions
// ------------------------------------------------------------------------------------------------

/// Inserts extern crate item into the AST of the original program.
pub fn insert_extern_crate_item(mut_mod: &mut ast::Mod, _inst_point: InstPoint) {
    dbg!("insert_extern_crate_item");
    // Insert use item for instrument crate
    mut_mod.items.insert(0, ptr::P(create_extern_crate_item()));
}

/// Inserts global scope initialization and finalization into the AST of the original program.
pub fn insert_global_scope(mut_item: &mut ast::Item, inst_point: InstPoint) {
    dbg!("insert global scope");
    if let ast::ItemKind::Fn(_, _, _, block) = &mut mut_item.node {
        // Insert actuall global init
        block.stmts.insert(0, create_global_init());
        // Insert local init for main thread
        block.stmts.insert(1, create_local_init());
        // Insert starting instrumentation call
        block.stmts.insert(
            2,
            create_instrumentation_call(&inst_point.static_data, DESCRIPTION_GLOBAL_BEGIN),
        );

        // The entire code

        // Insert ending instrumentation call
        block.stmts.push(create_instrumentation_call(
            &inst_point.static_data,
            DESCRIPTION_GLOBAL_END,
        ));
        // Insert local clean up for main thread
        block.stmts.push(create_local_clean_up());
    } else {
        // Should never be reached, InstFinder is resposible for ensuring correct type
        unreachable!()
    }
}

/// Inserts thread-local scope initialization and finalization into the AST of the original program.
pub fn insert_local_scope(mut_expr: &mut ast::Expr, inst_point: InstPoint) {
    dbg!("insert_local_scope");
    dbg!(inst_point.static_data.lines_begin);
    if let ast::ExprKind::Call(_expr_path, arg_exprs) = &mut mut_expr.node {
        for arg_expr in arg_exprs {
            if let ast::ExprKind::Closure(_, _, _, _, closure_expr, _) = &mut arg_expr.node {
                *closure_expr = ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Block(
                        ptr::P(ast::Block {
                            stmts: vec![
                                // Insert thread local init
                                create_local_init(),
                                // Insert thread local starting instrumentation call
                                create_instrumentation_call(
                                    &inst_point.static_data,
                                    DESCRIPTION_LOCAL_BEGIN,
                                ),
                                // Capture the entire closure body in a block and saved to a return var
                                ast::Stmt {
                                    id: DUMMY_NODE_ID,
                                    node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                                        NAME_OF_RETURN_VAR,
                                        "",
                                        Some(closure_expr.clone()),
                                    ))),
                                    span: DUMMY_SP,
                                },
                                // Insert thread local ending instrumentation call
                                create_instrumentation_call(
                                    &inst_point.static_data,
                                    DESCRIPTION_LOCAL_END,
                                ),
                                // Insert thread local clean up
                                // No clean_up needed, helper ends because channel sender ends
                                // create_local_clean_up(),
                                // Insert return of the original closure return value
                                create_return_value(),
                            ],
                            id: DUMMY_NODE_ID,
                            rules: ast::BlockCheckMode::Default,
                            span: DUMMY_SP,
                        }),
                        None,
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                });
            }
        }
        *mut_expr = ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Block(
                ptr::P(ast::Block {
                    stmts: vec![
                        // Instrumentation call
                        create_instrumentation_call(
                            &inst_point.static_data,
                            DESCRIPTION_INST_CALL_BEGIN,
                        ),
                        // Store result of original expression for later return
                        // "let value_2_return = original_expression;"
                        ast::Stmt {
                            id: DUMMY_NODE_ID,
                            node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                                NAME_OF_RETURN_VAR,
                                "",
                                Some(ptr::P(mut_expr.clone())),
                            ))),
                            span: DUMMY_SP,
                        },
                        // Instrumentation call
                        create_instrumentation_call(
                            &inst_point.static_data,
                            DESCRIPTION_INST_CALL_END,
                        ),
                        // Return result of original expression
                        // "NAME_OF_RETURN_VAR"
                        create_return_value(),
                    ],
                    id: DUMMY_NODE_ID,
                    // Safe block
                    rules: ast::BlockCheckMode::Default,
                    span: source_map::DUMMY_SP,
                }),
                None,
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        };
    } else if let ast::ExprKind::MethodCall(_expr_path, arg_exprs) = &mut mut_expr.node {
        for arg_expr in arg_exprs {
            if let ast::ExprKind::Closure(_, _, _, _, closure_expr, _) = &mut arg_expr.node {
                *closure_expr = ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Block(
                        ptr::P(ast::Block {
                            stmts: vec![
                                // Insert thread local init
                                create_local_init(),
                                // Insert thread local starting instrumentation call
                                create_instrumentation_call(
                                    &inst_point.static_data,
                                    DESCRIPTION_LOCAL_BEGIN,
                                ),
                                // Capture the entire closure body in a block and saved to a return var
                                ast::Stmt {
                                    id: DUMMY_NODE_ID,
                                    node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                                        NAME_OF_RETURN_VAR,
                                        "",
                                        Some(closure_expr.clone()),
                                    ))),
                                    span: DUMMY_SP,
                                },
                                // Insert thread local ending instrumentation call
                                create_instrumentation_call(
                                    &inst_point.static_data,
                                    DESCRIPTION_LOCAL_END,
                                ),
                                // Insert thread local clean up
                                // No clean_up needed, helper ends because channel sender ends
                                // create_local_clean_up(),
                                // Insert return of the original closure return value
                                create_return_value(),
                            ],
                            id: DUMMY_NODE_ID,
                            rules: ast::BlockCheckMode::Default,
                            span: DUMMY_SP,
                        }),
                        None,
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                });
            }
        }
        *mut_expr = ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Block(
                ptr::P(ast::Block {
                    stmts: vec![
                        // Instrumentation call
                        create_instrumentation_call(
                            &inst_point.static_data,
                            DESCRIPTION_INST_CALL_BEGIN,
                        ),
                        // Store result of original expression for later return
                        // "let value_2_return = original_expression;"
                        ast::Stmt {
                            id: DUMMY_NODE_ID,
                            node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                                NAME_OF_RETURN_VAR,
                                "",
                                Some(ptr::P(mut_expr.clone())),
                            ))),
                            span: DUMMY_SP,
                        },
                        // Instrumentation call
                        create_instrumentation_call(
                            &inst_point.static_data,
                            DESCRIPTION_INST_CALL_END,
                        ),
                        // Return result of original expression
                        // "NAME_OF_RETURN_VAR"
                        create_return_value(),
                    ],
                    id: DUMMY_NODE_ID,
                    // Safe block
                    rules: ast::BlockCheckMode::Default,
                    span: source_map::DUMMY_SP,
                }),
                None,
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        };
    } else {
        // Should never be reached, InstFinder is resposible for ensuring correct type
        unreachable!()
    }
}

/// Inserts instrumentation arround function calls.
pub fn insert_inst_call_function(mut_expr: &mut ast::Expr, inst_point: InstPoint) {
    dbg!("insert_inst_call_function");
    if let ast::ExprKind::Call(func, args_vec) = &mut mut_expr.node {
        // Extract arguments of interesting method call
        let (mut block, args_vars) = extract_arguments(args_vec.clone());
        let mut stmts_vec = vec![
            // Instrumentation call
            create_instrumentation_call(&inst_point.static_data, DESCRIPTION_INST_CALL_BEGIN),
            // Store result of original expression for later return
            // "let value_2_return = original_expression;"
            ast::Stmt {
                id: DUMMY_NODE_ID,
                node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                    NAME_OF_RETURN_VAR,
                    "",
                    Some(ptr::P(ast::Expr {
                        id: DUMMY_NODE_ID,
                        node: ast::ExprKind::Call(func.clone(), args_vars),
                        span: DUMMY_SP,
                        attrs: mut_expr.attrs.clone(),
                    })),
                ))),
                span: DUMMY_SP,
            },
            // Instrumentation call
            create_instrumentation_call(&inst_point.static_data, DESCRIPTION_INST_CALL_END),
            // Return result of original expression
            // "NAME_OF_RETURN_VAR"
            create_return_value(),
        ];
        block.append(&mut stmts_vec);

        // Assign new block
        *mut_expr = ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Block(
                ptr::P(ast::Block {
                    stmts: block,
                    id: DUMMY_NODE_ID,
                    // Safe block
                    rules: ast::BlockCheckMode::Default,
                    span: source_map::DUMMY_SP,
                }),
                None,
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        };
    } else {
        // Should never be reached, InstFinder is resposible for ensuring correct type
        unreachable!()
    }
}

/// Inserts instrumentation arround method calls.
pub fn insert_inst_call_method(mut_expr: &mut ast::Expr, inst_point: InstPoint) {
    dbg!("insert_inst_call_method");
    if let ast::ExprKind::MethodCall(path_s, args) = &mut mut_expr.node {
        // Unwind Method chain
        let called_on = args.remove(0).into_inner();
        let mut is_method_chain = false;

        let mut block = if let ast::ExprKind::MethodCall(_path, _args) = &called_on.node {
            is_method_chain = true;
            unwind_method_chain(called_on.clone())
        } else {
            is_method_chain = false;
            Vec::new()
        };
        // Extract arguments of interesting method call
        let (mut extracted_args, mut args_vars) = extract_arguments(args.clone());
        // Insert extracted arguments to block
        block.append(&mut extracted_args);
        // Instrumentation call
        block.push(create_instrumentation_call(
            &inst_point.static_data,
            DESCRIPTION_INST_CALL_BEGIN,
        ));
        // MethodCall of interest gets binded to return value
        block.push(ast::Stmt {
            id: DUMMY_NODE_ID,
            node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                NAME_OF_RETURN_VAR,
                "",
                Some(ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::MethodCall(path_s.clone(), {
                        let mut arguments = vec![{
                            if is_method_chain {
                                ptr::P(ast::Expr {
                                    id: DUMMY_NODE_ID,
                                    node: ast::ExprKind::Path(
                                        None,
                                        build_std_ast_path(NAME_OF_INTERMEDIATE_VAR, "0"),
                                    ),
                                    span: DUMMY_SP,
                                    attrs: ThinVec::new(),
                                })
                            } else {
                                ptr::P(called_on)
                            }
                        }];
                        // Pass extracted argument vars
                        arguments.append(&mut args_vars);
                        arguments
                    }),
                    span: mut_expr.span,
                    attrs: mut_expr.attrs.clone(),
                })),
            ))),
            span: DUMMY_SP,
        });
        // Instrumentation call
        block.push(create_instrumentation_call(
            &inst_point.static_data,
            DESCRIPTION_INST_CALL_END,
        ));
        // Return return_value
        block.push(create_return_value());

        // *Assign new block
        *mut_expr = ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Block(
                ptr::P(ast::Block {
                    stmts: block,
                    id: DUMMY_NODE_ID,
                    // Safe block
                    rules: ast::BlockCheckMode::Default,
                    span: source_map::DUMMY_SP,
                }),
                None,
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        };
        dbg!(&mut_expr);
    } else {
        // Should never be reached, InstFinder is resposible for ensuring correct type
        unreachable!()
    }
}

// Create functions
// ------------------------------------------------------------------------------------------------

/// Creates global init AST structure.
fn create_global_init() -> ast::Stmt {
    ast::Stmt {
        id: DUMMY_NODE_ID,
        node: ast::StmtKind::Semi(ptr::P(ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Call(
                ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Path(
                        None,
                        build_2_ast_path(NAME_OF_INST_CRATE, "", NAME_OF_GLOBAL_INIT_FN, ""),
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                }),
                Vec::new(),
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        })),
        span: DUMMY_SP,
    }
}

/// Creates local init AST structure.
fn create_local_init() -> ast::Stmt {
    ast::Stmt {
        id: DUMMY_NODE_ID,
        node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
            NAME_OF_LOCAL_THREAD_HANDLE_VAR,
            "",
            Some(ptr::P(ast::Expr {
                id: DUMMY_NODE_ID,
                node: ast::ExprKind::Call(
                    ptr::P(ast::Expr {
                        id: DUMMY_NODE_ID,
                        node: ast::ExprKind::Path(
                            None,
                            build_2_ast_path(NAME_OF_INST_CRATE, "", NAME_OF_LOCAL_INIT_FN, ""),
                        ),
                        span: DUMMY_SP,
                        attrs: ThinVec::new(),
                    }),
                    Vec::new(),
                ),
                span: DUMMY_SP,
                attrs: ThinVec::new(),
            })),
        ))),
        span: DUMMY_SP,
    }
}

/// Creates clean up AST structure.
fn create_local_clean_up() -> ast::Stmt {
    ast::Stmt {
        id: DUMMY_NODE_ID,
        node: ast::StmtKind::Semi(ptr::P(ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Call(
                ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Path(
                        None,
                        build_2_ast_path(NAME_OF_INST_CRATE, "", NAME_OF_LOCAL_CLEAN_UP_FN, ""),
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                }),
                vec![ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Path(
                        None,
                        build_std_ast_path(NAME_OF_LOCAL_THREAD_HANDLE_VAR, ""),
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                })],
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        })),
        span: DUMMY_SP,
    }
}

/// Creates instrumentation call AST structure.
fn create_instrumentation_call(static_data: &StaticData, description: &str) -> ast::Stmt {
    ast::Stmt {
        id: DUMMY_NODE_ID,
        node: ast::StmtKind::Semi(ptr::P(ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Call(
                ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Path(
                        None,
                        build_2_ast_path(NAME_OF_INST_CRATE, "", NAME_OF_INSTRUMENT_CALL_FN, ""),
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                }),
                vec![ptr::P(ast::Expr {
                    id: DUMMY_NODE_ID,
                    node: ast::ExprKind::Call(
                        ptr::P(ast::Expr {
                            id: DUMMY_NODE_ID,
                            node: ast::ExprKind::Path(
                                None,
                                ast::Path {
                                    span: DUMMY_SP,
                                    segments: vec![
                                        build_ast_pathsegment(NAME_OF_INST_CRATE, "", None),
                                        build_ast_pathsegment(NAME_OF_STATICDATA_STRUCT, "", None),
                                        build_ast_pathsegment(NAME_OF_STATICDATA_NEW_FN, "", None),
                                    ],
                                },
                            ),
                            span: DUMMY_SP,
                            attrs: ThinVec::new(),
                        }),
                        vec![
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Str(
                                        Symbol::intern(static_data.absolute_path.as_str()),
                                        ast::StrStyle::Cooked,
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Str(
                                        Symbol::intern(description),
                                        ast::StrStyle::Cooked,
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Int(
                                        static_data.ast_depth as u128,
                                        ast::LitIntType::Unsigned(ast::UintTy::U128),
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Str(
                                        Symbol::intern(static_data.source_file.as_str()),
                                        ast::StrStyle::Cooked,
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Int(
                                        static_data.lines_begin as u128,
                                        ast::LitIntType::Unsigned(ast::UintTy::U128),
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                            ptr::P(ast::Expr {
                                id: DUMMY_NODE_ID,
                                node: ast::ExprKind::Lit(source_map::Spanned {
                                    node: ast::LitKind::Int(
                                        static_data.lines_end as u128,
                                        ast::LitIntType::Unsigned(ast::UintTy::U128),
                                    ),
                                    span: DUMMY_SP,
                                }),
                                span: DUMMY_SP,
                                attrs: ThinVec::new(),
                            }),
                        ],
                    ),
                    span: DUMMY_SP,
                    attrs: ThinVec::new(),
                })],
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        })),
        span: DUMMY_SP,
    }
}

/// Creates return variable AST structure
fn create_return_value() -> ast::Stmt {
    ast::Stmt {
        id: DUMMY_NODE_ID,
        node: ast::StmtKind::Expr(ptr::P(ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Path(None, build_std_ast_path(NAME_OF_RETURN_VAR, "")),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        })),
        span: DUMMY_SP,
    }
}

/// Creates extern crate item
fn create_extern_crate_item() -> ast::Item {
    ast::Item {
        ident: build_ast_ident(NAME_OF_INST_CRATE, ""),
        attrs: Vec::new(),
        id: DUMMY_NODE_ID,
        node: ast::ItemKind::ExternCrate(None),
        vis: source_map::Spanned {
            node: ast::VisibilityKind::Inherited,
            span: DUMMY_SP,
        },
        span: DUMMY_SP,
        tokens: None,
    }
}

/// Extracts arguments of function and method calls 
fn extract_arguments(
    arguments: Vec<ptr::P<ast::Expr>>,
) -> (Vec<ast::Stmt>, Vec<ptr::P<ast::Expr>>) {
    let mut extracted_args = Vec::new();
    let mut args_vars = Vec::new();
    let mut var_count = 0;
    for expr in arguments {
        // Push Stmt local binding of argument to agrument_var
        extracted_args.push(ast::Stmt {
            id: DUMMY_NODE_ID,
            node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                NAME_OF_ARGUMENT_VAR,
                var_count.to_string().as_str(),
                Some(expr),
            ))),
            span: DUMMY_SP,
        });
        // Push Path aka variable into arguments vec for later actual function call
        args_vars.push(ptr::P(ast::Expr {
            id: DUMMY_NODE_ID,
            node: ast::ExprKind::Path(
                None,
                build_std_ast_path(NAME_OF_ARGUMENT_VAR, var_count.to_string().as_str()),
            ),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        }));

        var_count += 1;
    }
    // Return tuple
    (extracted_args, args_vars)
}

/// Unwinds a chained methods.
fn unwind_method_chain(expr: ast::Expr) -> Vec<ast::Stmt> {
    fn unwind_result_chain(
        rest_of_chain: ast::Expr,
        mut acc: Vec<ast::Stmt>,
        inter_var_num: i32,
    ) -> Vec<ast::Stmt> {
        match rest_of_chain.node {
            // Not for these expression kinds
            // these are all expressions which do not allow a method call directly on them
            ast::ExprKind::Box(..)
            | ast::ExprKind::ObsoleteInPlace(..)
            | ast::ExprKind::Binary(..)
            | ast::ExprKind::Unary(..)
            | ast::ExprKind::Cast(..)
            | ast::ExprKind::Type(..)
            | ast::ExprKind::Assign(..)
            | ast::ExprKind::AssignOp(..)
            | ast::ExprKind::Range(..)
            | ast::ExprKind::AddrOf(..)
            | ast::ExprKind::Break(..)
            | ast::ExprKind::Continue(..)
            | ast::ExprKind::Ret(..)
            | ast::ExprKind::InlineAsm(..)
            | ast::ExprKind::Try(..)
            | ast::ExprKind::Yield(..)
            | ast::ExprKind::Err => Vec::new(),

            // take method call and assign it to intermediate var
            ast::ExprKind::MethodCall(path_seg, mut args_vec) => {
                // Get first aka expression on which the method is called
                let rest = args_vec.remove(0).into_inner();
                // Update intermediate var name counter
                let new_inter_var_num = inter_var_num + 1;
                let mut last = vec![ast::Stmt {
                    id: DUMMY_NODE_ID,
                    node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                        NAME_OF_INTERMEDIATE_VAR,
                        inter_var_num.to_string().as_str(),
                        Some(ptr::P(ast::Expr {
                            id: DUMMY_NODE_ID,
                            node: ast::ExprKind::MethodCall(
                                // orig method call
                                path_seg,
                                {
                                    let mut arguments = vec![ptr::P(ast::Expr {
                                        id: DUMMY_NODE_ID,
                                        node: ast::ExprKind::Path(
                                            None,
                                            build_std_ast_path(
                                                NAME_OF_INTERMEDIATE_VAR,
                                                new_inter_var_num.to_string().as_str(),
                                            ),
                                        ),
                                        span: DUMMY_SP,
                                        attrs: ThinVec::new(),
                                    })];
                                    arguments.append(&mut args_vec);
                                    arguments
                                },
                            ),
                            span: DUMMY_SP,
                            attrs: rest_of_chain.attrs,
                        })),
                    ))),
                    span: DUMMY_SP,
                }];
                last.append(&mut acc);
                // Recursive function call
                unwind_result_chain(rest, last, new_inter_var_num)
            }

            // Base case: For every other expression kind which is left
            _ => {
                let mut last = vec![ast::Stmt {
                    id: DUMMY_NODE_ID,
                    node: ast::StmtKind::Local(ptr::P(build_std_ast_local_ident(
                        NAME_OF_INTERMEDIATE_VAR,
                        inter_var_num.to_string().as_str(),
                        Some(ptr::P(rest_of_chain)),
                    ))),
                    span: DUMMY_SP,
                }];
                last.append(&mut acc);
                // Retrun accumulated values
                last
            }
        }
    }
    // Call recursive helper function
    unwind_result_chain(expr, vec![], 0)
}

// Convenience functions

/// Convenience function builds a Path struct with single segment
fn build_std_ast_path(var_name: &str, var_counter: &str) -> ast::Path {
    ast::Path {
        span: DUMMY_SP,
        segments: vec![build_ast_pathsegment(var_name, var_counter, None)],
    }
}

/// Convenience function builds a Path with two segments
fn build_2_ast_path(var_n1: &str, var_c1: &str, var_n2: &str, var_c2: &str) -> ast::Path {
    ast::Path {
        span: DUMMY_SP,
        segments: vec![
            build_ast_pathsegment(var_n1, var_c1, None),
            build_ast_pathsegment(var_n2, var_c2, None),
        ],
    }
}

/// Convenience function builds a Pathsegment struct
fn build_ast_pathsegment(
    var_name: &str,
    var_counter: &str,
    tyargs: Option<ptr::P<ast::GenericArgs>>,
) -> ast::PathSegment {
    ast::PathSegment {
        id: DUMMY_NODE_ID,
        args: tyargs,
        ident: build_ast_ident(var_name, var_counter),
    }
}

/// Convenience function builds a Ident struct
fn build_ast_ident(var_name: &str, var_counter: &str) -> ast::Ident {
    ast::Ident {
        span: DUMMY_SP,
        name: Symbol::intern(format!("{}{}", var_name, var_counter).as_str()),
    }
}

/// Convenience function builds a Local struct
fn build_std_ast_local_ident(
    var_name: &str,
    var_counter: &str,
    init_expr: Option<ptr::P<ast::Expr>>,
) -> ast::Local {
    ast::Local {
        pat: ptr::P(ast::Pat {
            id: DUMMY_NODE_ID,
            node: ast::PatKind::Ident(
                ast::BindingMode::ByValue(ast::Mutability::Immutable),
                build_ast_ident(var_name, var_counter),
                None,
            ),
            span: DUMMY_SP,
        }),
        ty: None,
        // Original expression gets binded
        init: init_expr,
        id: DUMMY_NODE_ID,
        span: DUMMY_SP,
        attrs: ThinVec::new(),
    }
}
