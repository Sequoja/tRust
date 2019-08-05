use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use syntax::{ast, source_map, visit};

use crate::insertfuncs;
use instrument::StaticData;

/// Stores the reference to AST node and associated data for instrumentation.
#[derive(Debug)]
pub struct InstPoint<'p> {
    pub point: InstKind<'p>,
    pub absolute_path: String,
    pub ast_pos: (u128, u128, u128),
    pub static_data: StaticData,
}

impl<'p> InstPoint<'p> {
    /// Constructs new InstPoint.
    fn new(
        point: InstKind<'p>,
        absolute_path: String,
        ast_pos: (u128, u128, u128),
        static_data: StaticData,
    ) -> InstPoint<'p> {
        InstPoint {
            point,
            absolute_path,
            ast_pos,
            static_data,
        }
    }

    /// Calls the appropriate insert function.
    fn insert_inst(self) {
        match self.point {
            InstKind::ExternCrateItem(ast_ref) => unsafe {
                insertfuncs::insert_extern_crate_item(cast_point_2_mut(ast_ref).unwrap(), self)
            },
            InstKind::GlobalScope(ast_ref) => unsafe {
                insertfuncs::insert_global_scope(cast_point_2_mut(ast_ref).unwrap(), self)
            },
            InstKind::LocalScope(ast_ref) => unsafe {
                insertfuncs::insert_local_scope(cast_point_2_mut(ast_ref).unwrap(), self)
            },
            InstKind::InstCallForFunction(ast_ref) => unsafe {
                insertfuncs::insert_inst_call_function(cast_point_2_mut(ast_ref).unwrap(), self)
            },
            InstKind::InstCallForMethod(ast_ref) => unsafe {
                insertfuncs::insert_inst_call_method(cast_point_2_mut(ast_ref).unwrap(), self)
            },
        }
    }
}

impl<'p> Ord for InstPoint<'p> {
    fn cmp(&self, other: &InstPoint) -> Ordering {
        let depth = self.ast_pos.cmp(&other.ast_pos);
        if depth == Ordering::Equal {
            let path = self.absolute_path.cmp(&other.absolute_path);
            if path == Ordering::Equal {
                let inst_kind = self
                    .point
                    .get_string_rep()
                    .cmp(&other.point.get_string_rep());
                if inst_kind == Ordering::Equal {
                    self.static_data.lines_end.cmp(&other.static_data.lines_end)
                } else {
                    inst_kind
                }
            } else {
                path
            }
        } else {
            depth
        }
    }
}

impl<'p> PartialOrd for InstPoint<'p> {
    fn partial_cmp(&self, other: &InstPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'p> Eq for InstPoint<'p> {}

impl<'p> PartialEq for InstPoint<'p> {
    fn eq(&self, other: &InstPoint) -> bool {
        self.ast_pos == other.ast_pos
            && self.absolute_path == other.absolute_path
            && self.point.get_string_rep() == other.point.get_string_rep()
            && self.static_data.lines_end == other.static_data.lines_end
    }
}

/// Specifies the instrumentation kind for its contained AST node reference.
#[derive(Debug)]
pub enum InstKind<'p> {
    InstCallForFunction(&'p ast::Expr),
    InstCallForMethod(&'p ast::Expr),
    GlobalScope(&'p ast::Item),
    LocalScope(&'p ast::Expr),
    ExternCrateItem(&'p ast::Mod),
}

impl<'p> InstKind<'p> {
    /// Return String representation of the instrumentation kind
    fn get_string_rep(&self) -> String {
        match &self {
            InstKind::InstCallForFunction(_) => String::from("InstCallForFunction"),
            InstKind::InstCallForMethod(_) => String::from("InstCallForMethod"),
            InstKind::GlobalScope(_) => String::from("GlobalScope"),
            InstKind::LocalScope(_) => String::from("LocalScope"),
            InstKind::ExternCrateItem(_) => String::from("ExternCrateItem"),
        }
    }
}

/// Struct containes information associated with an AST node.
#[derive(Debug, Clone)]
struct PositionInfo {
    node_kind: String,
    filename: String,
    begin_line_col: (u128, u128),
    end_line_col: (u128, u128),
}

/// Inserts the instrumentation call at the specified positions.
pub struct InstFinder<'p> {
    resolv_paths: HashMap<String, String>,
    code_2_monitor: Vec<(String, String)>,
    code_2_monitor_names: HashSet<String>,
    inst_points: BTreeSet<InstPoint<'p>>,
    source_map: &'p source_map::SourceMap,
    ast_node_stack: VecDeque<PositionInfo>,
}

impl<'p> InstFinder<'p> {
    /// Constructor for the InstFinder struct.
    pub fn new(
        r_paths: HashMap<String, String>,
        c_2_m: Vec<(String, String)>,
        s_map: &'p source_map::SourceMap,
    ) -> InstFinder<'p> {
        let only_names: HashSet<String> = c_2_m.iter().map(|x| x.0.clone()).collect();
        InstFinder {
            resolv_paths: r_paths,
            code_2_monitor: c_2_m,
            code_2_monitor_names: only_names,
            inst_points: BTreeSet::new(),
            source_map: s_map,
            ast_node_stack: VecDeque::new(),
        }
    }

    /// Collects the InstPoints by walking the AST.
    pub fn find_inst_points(&mut self, krate: &'p ast::Crate) {
        visit::walk_crate(self, krate)
    }

    /// Inserts instrumentation for all collected InstPoints.
    pub fn insert_instrumentations(self) {
        for inst_point in self.inst_points.into_iter().rev() {
            inst_point.insert_inst();
        }
    }

    /// Getter for list of InstPoints.
    pub fn get_inst_points(&self) -> &BTreeSet<InstPoint<'p>> {
        &self.inst_points
    }

    /// Adds InstPoint to list if specified in config file
    fn add_point_if_needed(&mut self, path: String, point: InstKind<'p>, pos_info: PositionInfo) {
        if let Some((absolute_path, point_kinds)) = self.needs_inst(path, point.get_string_rep()) {
            // compute ast_depth here
            let ast_depth: u128 = self.ast_node_stack.len() as u128;

            if point_kinds.len() == 1 && point_kinds[0] == "LocalScope" {
                if let InstKind::InstCallForFunction(spawn) = point {
                    self.inst_points.insert(InstPoint::new(
                        InstKind::LocalScope(spawn),
                        absolute_path.clone(),
                        (
                            ast_depth,
                            pos_info.begin_line_col.0,
                            pos_info.begin_line_col.1,
                        ),
                        StaticData::new(
                            absolute_path.as_str(),
                            "",
                            ast_depth,
                            pos_info.filename.as_str(),
                            pos_info.begin_line_col.0,
                            pos_info.end_line_col.0,
                        ),
                    ));
                } else if let InstKind::InstCallForMethod(spawn) = point {
                    self.inst_points.insert(InstPoint::new(
                        InstKind::LocalScope(spawn),
                        absolute_path.clone(),
                        (
                            ast_depth,
                            pos_info.begin_line_col.0,
                            pos_info.begin_line_col.1,
                        ),
                        StaticData::new(
                            absolute_path.as_str(),
                            "",
                            ast_depth,
                            pos_info.filename.as_str(),
                            pos_info.begin_line_col.0,
                            pos_info.end_line_col.0,
                        ),
                    ));
                }
            } else if point_kinds.contains(&point.get_string_rep()) {
                self.inst_points.insert(
                    // BTreeMap Value
                    InstPoint::new(
                        // InstKind
                        point,
                        absolute_path.clone(),
                        // (ast_depth, begin line, begin column)
                        (
                            ast_depth,
                            pos_info.begin_line_col.0,
                            pos_info.begin_line_col.1,
                        ),
                        // StaticData
                        StaticData::new(
                            // Absolute path
                            absolute_path.as_str(),
                            // Description
                            "",
                            // AST depth
                            ast_depth,
                            // File name
                            pos_info.filename.as_str(),
                            // Begin line
                            pos_info.begin_line_col.0,
                            // End line
                            pos_info.end_line_col.0,
                        ),
                    ),
                );
            }
        }
    }

    /// Checks if an expression needs instrumentation.
    fn needs_inst(&mut self, path: String, str_inst_kind: String) -> Option<(String, Vec<String>)> {
        let absolute_path = self.determine_abs_path(path, str_inst_kind);

        if self.code_2_monitor_names.contains(&absolute_path) {
            let point_kinds = self.get_from_code_2_moditor(&absolute_path);
            Some((absolute_path, point_kinds))
        } else {
            None
        }
    }

    /// Resolves the absolute path of a name.
    fn determine_abs_path(&self, path: String, str_inst_kind: String) -> String {
        if str_inst_kind == "InstCallForMethod" {
            path
        } else {
            let segments: Vec<String> = path.split("::").map(|e| e.to_string()).collect();
            let beginning_of_path = self.resolv_paths.get(segments.first().unwrap());

            if beginning_of_path.is_some() {
                (&segments[1..])
                    .iter()
                    .fold(beginning_of_path.unwrap().clone(), |acc, x| {
                        (acc + "::") + x
                    })
            } else {
                segments.iter().fold("".to_string(), |acc, x| {
                    if acc.is_empty() {
                        acc + x
                    } else {
                        (acc + "::") + x
                    }
                })
            }
        }
    }

    /// Returns source file name and line number of current ast node.
    fn get_file_lines(&self, node_kind: String, span: source_map::Span) -> PositionInfo {
        if let Ok(filelines) = self.source_map.span_to_lines(span) {
            if let (Some(begin), Some(end)) = (filelines.lines.first(), filelines.lines.last()) {
                let source_map::CharPos(begin_col) = begin.start_col;
                let source_map::CharPos(end_col) = end.end_col;
                PositionInfo {
                    node_kind,
                    filename: filelines.file.name.to_string(),
                    begin_line_col: ((begin.line_index as u128) + 1, begin_col as u128),
                    end_line_col: ((end.line_index as u128) + 1, end_col as u128),
                }
            } else {
                PositionInfo {
                    node_kind,
                    filename: String::from(""),
                    begin_line_col: (0, 0),
                    end_line_col: (0, 0),
                }
            }
        } else {
            PositionInfo {
                node_kind,
                filename: String::from(""),
                begin_line_col: (0, 0),
                end_line_col: (0, 0),
            }
        }
    }

    /// Puts AST structure in list for later AST depth caluclation.
    fn set_ast_stack(&mut self, pos_info: PositionInfo) {
        let mut index = self.ast_node_stack.len();
        for x in self.ast_node_stack.iter().rev() {
            if x.filename == pos_info.filename
                && x.begin_line_col <= pos_info.begin_line_col
                && x.end_line_col >= pos_info.end_line_col
            {
                break;
            }
            index -= 1;
        }
        let _ = self.ast_node_stack.split_off(index);
        self.ast_node_stack.push_back(pos_info);
    }

    /// Filters code_2_monitor list
    fn get_from_code_2_moditor(&self, absolute_path: &str) -> Vec<String> {
        self.code_2_monitor
            .iter()
            .filter_map(|x| {
                if x.0 == absolute_path {
                    Some(x.1.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Finds AST nodes where methods and functions of interest are invoced
/// But invocations in macros are currently collected
impl<'p> visit::Visitor<'p> for InstFinder<'p> {
    fn visit_mod(
        &mut self,
        m: &'p ast::Mod,
        s: source_map::Span,
        _attrs: &[ast::Attribute],
        _n: ast::NodeId,
    ) {
        let pos_info = self.get_file_lines(String::from("module"), s);
        self.set_ast_stack(pos_info.clone());
        self.add_point_if_needed(String::from(""), InstKind::ExternCrateItem(m), pos_info);
        visit::walk_mod(self, m);
    }

    fn visit_item(&mut self, i: &'p ast::Item) {
        let pos_info = self.get_file_lines(String::from("item"), i.span);
        self.set_ast_stack(pos_info.clone());
        self.add_point_if_needed(i.ident.to_string(), InstKind::GlobalScope(i), pos_info);
        visit::walk_item(self, i);
    }

    fn visit_block(&mut self, b: &'p ast::Block) {
        let pos_info = self.get_file_lines(String::from("block"), b.span);
        self.set_ast_stack(pos_info);
        visit::walk_block(self, b);
    }

    fn visit_stmt(&mut self, s: &'p ast::Stmt) {
        let pos_info = self.get_file_lines(String::from("statement"), s.span);
        self.set_ast_stack(pos_info);
        visit::walk_stmt(self, s);
    }

    /// Actually mutates the AST
    fn visit_expr(&mut self, expr: &'p ast::Expr) {
        let pos_info = self.get_file_lines(String::from("expression"), expr.span);
        self.set_ast_stack(pos_info.clone());
        match &expr.node {
            // Function call (func_call(...) or path::func_call(...) or etc.)
            ast::ExprKind::Call(expr_path, _args) => {
                if let ast::ExprKind::Path(_qualified, path) = &expr_path.node {
                    self.add_point_if_needed(
                        path.to_string(),
                        InstKind::InstCallForFunction(expr),
                        pos_info,
                    );
                }
            }

            // Method call (var.method_call(...))
            ast::ExprKind::MethodCall(ast::PathSegment { ident, .. }, _) => {
                self.add_point_if_needed(
                    ident.to_string(),
                    InstKind::InstCallForMethod(expr),
                    pos_info,
                );
            }
            _ => (),
        }

        visit::walk_expr(self, expr);
    }

    fn visit_fn(
        &mut self,
        fk: visit::FnKind<'p>,
        fd: &'p ast::FnDecl,
        s: source_map::Span,
        _: ast::NodeId,
    ) {
        let pos_info = self.get_file_lines(String::from("function"), s);
        self.set_ast_stack(pos_info);
        visit::walk_fn(self, fk, fd, s);
    }

    fn visit_trait_item(&mut self, ti: &'p ast::TraitItem) {
        let pos_info = self.get_file_lines(String::from("trait_item"), ti.span);
        self.set_ast_stack(pos_info);
        visit::walk_trait_item(self, ti)
    }

    fn visit_impl_item(&mut self, ii: &'p ast::ImplItem) {
        let pos_info = self.get_file_lines(String::from("impl_item"), ii.span);
        self.set_ast_stack(pos_info);
        visit::walk_impl_item(self, ii)
    }

    fn visit_local(&mut self, l: &'p ast::Local) {
        let pos_info = self.get_file_lines(String::from("local"), l.span);
        self.set_ast_stack(pos_info);
        visit::walk_local(self, l)
    }

    fn visit_mac(&mut self, _mac: &'p ast::Mac) {
        // panic!("visit_mac disabled by default");
        // N.B., see note about macros above.
        // if you really want a visitor that
        // works on macros, use this
        // definition in your trait impl:
        let pos_info = self.get_file_lines(String::from("macro"), _mac.span);
        self.set_ast_stack(pos_info);
        visit::walk_mac(self, _mac);
    }
}

/// Converts a immutable reference to a mutable reference
unsafe fn cast_point_2_mut<A>(item_ref: &A) -> Option<&mut A> {
    ((item_ref as *const A) as *mut A).as_mut()
}
