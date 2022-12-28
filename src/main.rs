use colored::*; // for printing colored text to the terminal (for visualizing the simulation)
use inquire::{Confirm, Text};
use std::str::FromStr; // For prompting the user for input through the terminal

// use std::thread::sleep;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng}; // random numbers
use std::time::{Duration, Instant};

// ====================
//   GLOBAL CONSTANTS
// ====================
const WIDTH_DEFAULT: usize = 512;
const HEIGHT_DEFAULT: usize = 512;
const FRAMERATE_DEFAULT: usize = 5;
const SHOW_WHILE_RUNNING_DEFAULT: bool = false;
const COLORSHIFT_DEFAULT: u8 = 4;
const STARTING_LIVE_CELLS_DEFAULT: u32 = 1;
const SPREAD_CHANCE_DEFAULT: f64 = 0.5;

const LIVE_CELL_CHAR: char = '█';

#[derive(Debug, Clone)]
struct Grid {
    alive_states: Vec<Vec<bool>>,
    red_states: Vec<Vec<u8>>,
    green_states: Vec<Vec<u8>>,
    blue_states: Vec<Vec<u8>>,

    width: usize,
    height: usize,
    frametime: Duration,
    colorshift: u8,
    cell_char: String,
    spread_chance: f64,
}

impl Grid {
    /// Prints the grid to the terminal
    fn show(&self) {
        for i in 1..(self.height - 1) {
            for j in 1..(self.width - 1) {
                let red = self.red_states[i][j];
                let green = self.green_states[i][j];
                let blue = self.blue_states[i][j];
                print!("{}", self.cell_char.truecolor(red, green, blue));
            }
            println!();
        }
    }

    /// Shifts a color value using a ThreadRng and some shift value
    fn shift_color(x: u8, rng: &mut ThreadRng, shift: u8) -> u8 {
        let r = rng.gen_range(0..shift);
        if rng.gen_bool(0.5) {
            x.saturating_sub(r)
        } else {
            x.saturating_add(r)
        }
    }

    /// Makes a cell reproduce
    fn make_child(&mut self, x: usize, y: usize, new_x: usize, new_y: usize, rng: &mut ThreadRng) {
        let red = self.red_states[y][x];
        let green = self.green_states[y][x];
        let blue = self.blue_states[y][x];

        // Shift each color randomly
        let new_red = Grid::shift_color(red, rng, self.colorshift);
        let new_green = Grid::shift_color(green, rng, self.colorshift);
        let new_blue = Grid::shift_color(blue, rng, self.colorshift);

        // Place cell
        self.alive_states[new_y][new_x] = true;
        self.red_states[new_y][new_x] = new_red;
        self.green_states[new_y][new_x] = new_green;
        self.blue_states[new_y][new_x] = new_blue;
    }

    // Places a cell with a random color at a random position on the grid
    fn spawn_orphan_at_random_position(&mut self, rng: &mut ThreadRng) {
        // Index of new orphan cell
        let x = rng.gen_range(1..(self.width - 1));
        let y = rng.gen_range(1..(self.height - 1));

        // Place cell
        self.alive_states[y][x] = true;
        self.red_states[y][x] = rng.gen();
        self.green_states[y][x] = rng.gen();
        self.blue_states[y][x] = rng.gen();
    }

    // Checks all eight orthogonal neighbors of a cell and returns their x and y indices in the grid
    fn dead_nbors(&self, x: usize, y: usize) -> (Vec<usize>, Vec<usize>) {
        let mut dead_nbors_x = vec![];
        let mut dead_nbors_y = vec![];
        for yy in [y - 1, y, y + 1] {
            for xx in [x - 1, x, x + 1] {
                let dead = !self.alive_states[yy][xx];
                if ((yy, xx) != (y, x)) && dead {
                    dead_nbors_x.push(xx);
                    dead_nbors_y.push(yy);
                }
            }
        }
        (dead_nbors_x, dead_nbors_y)
    }
}

fn main() {
    // Attempts to convert a string into the desired type T
    fn parse<T: FromStr>(s: String, default_value: T) -> T {
        s.to_lowercase().parse::<T>().unwrap_or(default_value)
    }

    // Prompts the user and converts the entered string into the desired type T
    // If an error occurs or the string cannot be parsed into the desired type, default_value is returned
    fn parsed_prompt<T: FromStr>(prompt: &str, default_value: T, skippable: bool) -> T {
        let x = Text::new(prompt); // create the prompt

        match skippable {
            true => match x.prompt_skippable() {
                Ok(Some(s)) => parse(s, default_value),
                _ => default_value,
            },
            false => match x.prompt() {
                Ok(s) => parse(s, default_value),
                _ => default_value,
            },
        }
    }

    // PROMPT USER FOR ALL ARGUMENTS FOR THE SIMULATION
    // Most of these prompts are skippable and have default values they can fall back to
    let (
        width,
        height,
        starting_live_cells,
        framerate,
        show_while_running,
        colorshift,
        spread_chance,
    ) = match Confirm::new("Run with default settings?")
        .prompt()
        .unwrap_or(true)
    {
        true => (
            WIDTH_DEFAULT,
            HEIGHT_DEFAULT,
            STARTING_LIVE_CELLS_DEFAULT,
            FRAMERATE_DEFAULT,
            SHOW_WHILE_RUNNING_DEFAULT,
            COLORSHIFT_DEFAULT,
            SPREAD_CHANCE_DEFAULT,
        ),
        false => (
            parsed_prompt("Enter Width in pixels", WIDTH_DEFAULT, true),
            parsed_prompt("Enter Height in pixels", HEIGHT_DEFAULT, true),
            parsed_prompt(
                "Enter the number of Starting Live Cells",
                STARTING_LIVE_CELLS_DEFAULT,
                true,
            ),
            parsed_prompt("Enter framerate", FRAMERATE_DEFAULT, true),
            SHOW_WHILE_RUNNING_DEFAULT, // CHANGE THIS LATER
            parsed_prompt("Enter colorshift value", COLORSHIFT_DEFAULT, true),
            parsed_prompt(
                "Enter spreadchance (0.0 -> 1.0)",
                SPREAD_CHANCE_DEFAULT,
                true,
            ),
        ),
    };

    let mut rng = thread_rng(); // random number generator
    let now = Instant::now(); // Begin timing the program
    let frametime = Duration::from_millis((1000.0 / (framerate as f64)).trunc() as u64); // the amount of time that the animation sleeps between frames to keep a constant framerate

    // ============================
    //          CREATE GRID
    // ============================

    let mut grid = Grid {
        alive_states: vec![vec![false; width]; height],
        red_states: vec![vec![0; width]; height],
        green_states: vec![vec![0; width]; height],
        blue_states: vec![vec![0; width]; height],
        width,
        height,
        frametime,
        colorshift,
        cell_char: String::from(LIVE_CELL_CHAR),
        spread_chance,
    };

    // =======================
    //  PLACE STARTING CELLS
    // =======================
    for _ in 0..starting_live_cells {
        grid.spawn_orphan_at_random_position(&mut rng);
    }

    /*
     ____ ___ __  __ _   _ _        _  _____ ___ ___  _   _
    / ___|_ _|  \/  | | | | |      / \|_   _|_ _/ _ \| \ | |
    \___ \| || |\/| | | | | |     / _ \ | |  | | | | |  \| |
     ___) | || |  | | |_| | |___ / ___ \| |  | | |_| | |\  |
    |____/___|_|  |_|\___/|_____/_/   \_\_| |___\___/|_| \_|
    */

    // Produces all the indices of a Vec<Vec<_>> with some width and height
    // Height is the .len() of the outer vec
    // Width is the .len() of the inner vec
    fn get_coordinate_pairs(width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut pairs = Vec::with_capacity(width * height);
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                pairs.push((y, x));
            }
        }
        pairs
    }

    // In theory this would improve performance. In practice it does not.
    let yx_coordinate_pairs = get_coordinate_pairs(width, height);

    println!("Any pre-caching has finished.\nBeginning Simulation...");
    if show_while_running {
        // Show the grid as cells spread (pretty but also slower)
        todo!();
        // loop {

        // }
    } else {
        // Only show the resulting art after its finished rendering (much faster!)
        println!("Running in background");
        loop {
            let mut seen_dead_cell = false;
            // UPDATE SPREADABLE CELLS
            // for y in 1..(height - 1) {
            //     for x in 1..(width - 1) {
            //         if grid.alive_states[y][x] {
            //             let (dead_x, dead_y) = grid.dead_nbors(x, y);
            //             if (!dead_x.is_empty()) && (rng.gen_bool(spread_chance)) {
            //                 let random_nbor_index = rng.gen_range(0..dead_x.len());
            //                 let new_x = dead_x[random_nbor_index];
            //                 let new_y = dead_y[random_nbor_index];
            //                 grid.make_child(x, y, new_x, new_y, &mut rng);
            //             }
            //         } else {
            //             seen_dead_cell = true;
            //         }
            //     }
            // }

            for (y, x) in yx_coordinate_pairs.iter() {
                let (y, x) = (*y, *x);
                if grid.alive_states[y][x] {
                    let (dead_x, dead_y) = grid.dead_nbors(x, y);
                    if (!dead_x.is_empty()) && (rng.gen_bool(spread_chance)) {
                        let random_nbor_index = rng.gen_range(0..dead_x.len());
                        let new_x = dead_x[random_nbor_index];
                        let new_y = dead_y[random_nbor_index];
                        grid.make_child(x, y, new_x, new_y, &mut rng);
                    }
                } 
                else {
                    seen_dead_cell = true;
                }
            }

            if !seen_dead_cell {
                break;
            }
        }
    }

    // ===============================
    // ===============================
    //          PRINT RESULTS
    // ===============================
    // ===============================
    clearscreen::clear().unwrap();
    println!("Finished in {:?}", now.elapsed());

    if Confirm::new("Preview final image in the terminal?")
        .prompt()
        .unwrap_or(true)
    {
        // Show the final result in the terminal if desired
        grid.show();
    }

    if Confirm::new("Save final animation frame as an image?")
        .prompt()
        .unwrap_or(true)
    {
        // otherwise save the result as a file
        let default_filename = "image.png".to_string();
        let filename = Text::new("Enter a filename for your picture")
            .prompt()
            .unwrap_or(default_filename);

        let img_timer = Instant::now();
        // save the result as an image using the `image` crate
        let img = image::ImageBuffer::from_fn(
            width.try_into().unwrap(),
            height.try_into().unwrap(),
            |x, y| {
                let red = grid.red_states[y as usize][x as usize];
                let green = grid.green_states[y as usize][x as usize];
                let blue = grid.blue_states[y as usize][x as usize];
                image::Rgb([red, green, blue])
            },
        );
        match img.save(filename) {
            Err(e) => println!(
                "Sorry, the file wasn't able to because of this error -> {:?}",
                e
            ),
            _ => println!(
                "Finished generating and saving image in {:?}",
                img_timer.elapsed()
            ),
        }
    }
}
