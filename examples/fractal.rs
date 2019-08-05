use rayon::prelude::*;

// fn julia_set(set: Vec<(i64, i64)>) -> i64 {
//     for (x,y) in set {
//         let zx = x;
//         let zy = y;

//         iteration = 0;
//         max_iterations 1_000_000;

//         while zx * zx + zy * zy < 4  &&  iteration < max_iteration {
//             let xtemp = zx * zx - zy * zy;
//             zy = 2 * zx * zy  + cy;
//             zx = xtemp + cx;

//             iteration = iteration + 1;
//         }

//         if iteration == max_iteration {
//             return 0;
//         } else {
//             return iteration;
//         }
//     }
// }

fn julia_set(screen: &mut [Vec<char>], max_iteration: u128) {
    let hight = screen.len();
    let width = screen[0].len();
    screen.par_iter_mut().enumerate().for_each(|re| {
        let r_index = re.0;
        re.1.iter_mut().enumerate().for_each(|e| {
            let c_index = e.0;
            let c_re: f64 = (c_index as f64 - width as f64 / 2.0) * 4.0 / width as f64;
            let c_im: f64 = (r_index as f64 - hight as f64 / 2.0) * 4.0 / width as f64;
            let mut x = 0.0;
            let mut y = 0.0;
            let mut iterations = 0;
            while x * x + y * y < 4.0 && iterations < max_iteration {
                let new_x: f64 = x * x - y * y + c_re;
                y = 2.0 * x * y + c_im;
                x = new_x;
                iterations += 1;
            }
            if iterations < max_iteration {
                *e.1 = ' ';
            } else {
                *e.1 = '#';
            }
        });
    });
}

fn build_screen_vec(size: usize) -> Vec<Vec<char>> {
    vec![vec![' '; 2 * size]; size]
}

fn print_screen(screen: &[Vec<char>]) {
    screen.iter().for_each(|row| {
        row.iter().for_each(|elem_in_col| print!("{}", elem_in_col));
        println!("")
    });
    println!("")
}

fn main() {
    let max_iteration: u128 = 1_000_000;
    let mut screen = build_screen_vec(96);
    julia_set(&mut screen, max_iteration);
    // println!("{:?}", screen);
    print_screen(&screen);
}
