use colored::*; // for printing colored text to the terminal (for visualizing the simulation)
use inquire::{Confirm, Text}; // For prompting the user for input through the terminal

use std::{
    fmt::Display,
    str::FromStr,
    time::{Duration, Instant},
};

use ndarray::Array2;

use rand::{rngs::ThreadRng, seq::IteratorRandom, thread_rng, Rng};

// ====================
//   GLOBAL CONSTANTS
// ====================

// DEFAULTS (if user skips prompts these defaults are used in place of user input)
const WIDTH_DEFAULT: usize = 32;
const HEIGHT_DEFAULT: usize = 16;
const FRAMERATE_DEFAULT: usize = 32;
const SHOW_WHILE_RUNNING_DEFAULT: bool = false;
const COLORSHIFT_DEFAULT: u8 = 4;
const STARTING_LIVE_CELLS_DEFAULT: u32 = 1;
const SPREAD_CHANCE_DEFAULT: f64 = 0.5;

const LIVE_CELL_CHAR: char = '█'; // character used to represent 'live' cells

#[derive(Debug, Clone, Copy)]
struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl RgbColor {
    fn as_slice(&self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }

    // Returns a random color
    fn random(rng: &mut ThreadRng) -> Self {
        Self {
            red: rng.gen(),
            green: rng.gen(),
            blue: rng.gen(),
        }
    }

    // Increases or decreases a color value (red, green, or blue) given a shift value
    fn shift_hue(hue: u8, shift: u8, rng: &mut ThreadRng) -> u8 {
        let r = rng.gen_range(0..shift);
        if rng.gen() {
            hue.saturating_sub(r)
        } else {
            hue.saturating_add(r)
        }
    }

    /// Shifts each of a color's Red, Green, and Blue values randomly,
    /// given a `shift` value and a random number generator
    fn shift_color(&self, shift: u8, rng: &mut ThreadRng) -> Self {
        Self {
            red: RgbColor::shift_hue(self.red, shift, rng),
            green: RgbColor::shift_hue(self.green, shift, rng),
            blue: RgbColor::shift_hue(self.blue, shift, rng),
        }
    }
}

impl From<[u8; 3]> for RgbColor {
    fn from(value: [u8; 3]) -> Self {
        let [red, green, blue] = value;
        Self { red, green, blue }
    }
}

#[derive(Debug, Clone)]
struct Grid {
    alive_states: Array2<bool>,
    // red_states: Array2<u8>,
    // green_states: Array2<u8>,
    // blue_states: Array2<u8>,
    color_states: Array2<RgbColor>,

    // Dimensions of the simulation
    width: usize,
    height: usize,

    /*
    Time to sleep between frames
    (this only matters if the simulation is being animated in the terminal.)
    */
    frametime: Duration,

    /*
    When a living cell spreads to a dead cell,
    the new cell's color is generated using
    the parent's color and this value
    */
    colorshift: u8,
    cell_char: String,
    spread_chance: f64,
}

impl Grid {
    /// Prints the grid to the terminal
    fn show(&self) {
        for y in 1..(self.height - 1) {
            for x in 1..(self.width - 1) {
                // let [red, green, blue] = self.get_color(y, x).as_slice();
                // print!("{}", self.cell_char.truecolor(*red, *green, *blue));
                print!("{}", self.get_cell_on_its_color(y, x))
            }
            println!();
        }
    }

    fn get_color(&self, y: usize, x: usize) -> RgbColor {
        self.color_states[[y, x]]
    }

    fn set_color(&mut self, y: usize, x: usize, color: RgbColor) {
        self.color_states[[y, x]] = color;
        // println!("Setting {y} {x} to {color:?}");
    }

    // Returns a String representing a cell displayed in its color
    fn get_cell_on_its_color(&self, y: usize, x: usize) -> ColoredString {
        let [r, g, b] = self.get_color(y, x).as_slice();
        self.cell_char.truecolor(r, g, b)
    }

    // Prints a message saying that this cell spread somewhere
    fn spread_message(&self, y: usize, x: usize, new_y: usize, new_x: usize) {
        let parent = self.get_cell_on_its_color(y, x);
        let child = self.get_cell_on_its_color(new_y, new_x);
        println!(
            "{parent} / {:?} {y},{x} spread to {child} / {:?} {new_y},{new_x}",
            self.get_color(y, x).as_slice(),
            self.get_color(new_y, new_x).as_slice()
        );
    }

    /// Makes a cell reproduce
    fn make_child(&mut self, y: usize, x: usize, new_y: usize, new_x: usize, rng: &mut ThreadRng) {
        // let [red, green, blue] = self.get_color(y, x);

        // Shift each color randomly
        // let new_red = RGB_Color::shift_color();
        // let new_green = RGB_Color::shift_color(green, rng, self.colorshift);
        // let new_blue = RGB_Color::shift_color(blue, rng, self.colorshift);

        // Get current color, and shift each of its color channels randomly using self.colorshift
        let current_color = self.get_color(y, x);
        let new_color: RgbColor = current_color.shift_color(self.colorshift, rng);

        // Place cell
        self.alive_states[[new_y, new_x]] = true;
        self.set_color(new_y, new_x, new_color);
        // println!("Cell at [{y} {x}] with Color {color_slice:?} spread to [{new_y} {new_x}] w/ Color ({new_color_slice:?})");
        // self.spread_message(y, x, new_y, new_x);
    }

    // Places a cell with a random color at a random position on the grid
    fn spawn_orphan_at_random_position(&mut self, rng: &mut ThreadRng) {
        // Index of new orphan cell
        let x = rng.gen_range(1..(self.width - 1));
        let y = rng.gen_range(1..(self.height - 1));

        // Place cell
        self.alive_states[[y, x]] = true;
        let color = RgbColor::random(rng);
        self.set_color(y, x, color);
        let [red, green, blue] = color.as_slice();
        let color_str = self.cell_char.truecolor(red, green, blue);
        println!("Spawning orphan {color_str} @ {y},{x}");
    }

    // Checks all eight orthogonal neighbors of a cell and returns their x and y indices in the grid
    fn spread_to_random_dead_nbor(&mut self, y: usize, x: usize, rng: &mut ThreadRng) {
        if let Some([new_y, new_x]) = [
            [y - 1, x - 1],
            [y - 1, x],
            [y - 1, x + 1],
            [y, x - 1],
            [y, x + 1],
            [y + 1, x - 1],
            [y + 1, x],
            [y + 1, x + 1],
        ]
        .into_iter()
        .filter(|ind| !self.alive_states[*ind])
        .choose(rng)
        {
            if rng.gen_range(0.0..1.0) < self.spread_chance {
                self.make_child(y, x, new_y, new_x, rng);
            }
        }
    }
}

// Same as parsed prompt, but this prompt is skippable.
// If the prompt is skipped or the user's input cannot be parsed as type T, then default_value is returned.
fn parsed_prompt_skippable<T: FromStr + Display>(prompt: &str, default_value: T) -> T {
    Text::new(prompt)
        .with_default(&default_value.to_string())
        .prompt_skippable()
        .expect("parsed_prompt failed to parse prompt")
        .unwrap()
        .to_lowercase()
        .parse::<T>()
        .unwrap_or(default_value)
}

fn confirm_skippable(prompt: &str, default: bool) -> bool {
    Confirm::new(prompt).with_default(default).prompt().unwrap()
}

fn main() {
    // ==============================
    //     SET SIMULATION SETTINGS
    // ==============================
    let (
        width,
        height,
        starting_live_cells,
        framerate,
        show_while_running,
        colorshift,
        spread_chance,
    ) = if Confirm::new("Run with default settings?")
        .prompt()
        .unwrap_or(true)
    {
        (
            WIDTH_DEFAULT,
            HEIGHT_DEFAULT,
            STARTING_LIVE_CELLS_DEFAULT,
            FRAMERATE_DEFAULT,
            SHOW_WHILE_RUNNING_DEFAULT,
            COLORSHIFT_DEFAULT,
            SPREAD_CHANCE_DEFAULT,
        )
    } else {
        (
            parsed_prompt_skippable("Enter Width in pixels", WIDTH_DEFAULT),
            parsed_prompt_skippable("Enter Height in pixels", HEIGHT_DEFAULT),
            parsed_prompt_skippable(
                "Enter the number of Starting Live Cells",
                STARTING_LIVE_CELLS_DEFAULT,
            ),
            parsed_prompt_skippable("Enter framerate", FRAMERATE_DEFAULT),
            confirm_skippable(
                "Animate in the terminal while running?",
                SHOW_WHILE_RUNNING_DEFAULT,
            ),
            parsed_prompt_skippable("Enter colorshift value", COLORSHIFT_DEFAULT),
            parsed_prompt_skippable("Enter spreadchance (0.0 -> 1.0)", SPREAD_CHANCE_DEFAULT),
        )
    };

    let now = Instant::now(); // Begin timing the program
    let frametime = {
        let frame_rate: u64 = framerate.try_into().unwrap();
        Duration::from_micros(1_000_000 / frame_rate)
    }; // the amount of time that the animation sleeps between frames to keep a constant framerate

    let grid_shape = [height, width];
    let mut grid = Grid {
        alive_states: Array2::from_elem(grid_shape, false),
        // red_states: Array2::zeros(grid_shape),
        // green_states: Array2::zeros(grid_shape),
        // blue_states: Array2::zeros(grid_shape),
        color_states: Array2::from_elem(
            grid_shape,
            RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
        ),
        width,
        height,
        frametime,
        colorshift,
        cell_char: LIVE_CELL_CHAR.to_string(),
        spread_chance,
    };

    // =======================
    //  PLACE STARTING CELLS
    // =======================
    let mut rng = thread_rng();
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
    // In theory this would improve performance. In practice it does not.
    let mut yx_coordinate_pairs = Vec::with_capacity(width * height);
    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            yx_coordinate_pairs.push([y, x]);
        }
    }
    // Make immutable, since it will never be modified again.
    yx_coordinate_pairs.shrink_to_fit();

    // ANIMATE or RUN IN BACKGROUND
    // Depending on what the user decided earlier.
    let final_grid: Grid = if show_while_running {
        // simulation_animated(grid, &yx_coordinate_pairs)
        todo!();
    } else {
        simulation_in_background(grid, &yx_coordinate_pairs)
    };

    // Print results
    println!("Finished in {:?}", now.elapsed());
    save_results(final_grid);
}

fn save_results(grid: Grid) {
    // Show the final result in the terminal if desired
    if confirm_skippable("Preview final image in terminal?", false) {
        grid.show();
    }

    // Save final result as an image if desired
    if confirm_skippable("Save final frame as an image?", false) {
        let filename = Text::new("Enter a filename for your picture")
            .prompt()
            .unwrap_or("image.png".to_string());

        let img_timer = Instant::now();
        // save the result as an image using the `image` crate
        let img = image::ImageBuffer::from_fn(
            grid.width.try_into().unwrap(),
            grid.height.try_into().unwrap(),
            |y, x| {
                let y: usize = y.try_into().unwrap();
                let x: usize = x.try_into().unwrap();
                image::Rgb(grid.get_color(y, x).as_slice())
            },
        );
        if let Err(e) = img.save(&format!("output_images/{filename}")) {
            println!("Sorry, the file wasn't able to because of this error -> {e:?}")
        } else {
            println!(
                "Finished generating and saving image in {:?}",
                img_timer.elapsed()
            );
            println!("{filename} was saved in the output_images directory");
        }
    }
}

// Runs the simulation without visualizing it in the terminal.
// This is faster, and helpful if you only want the final output image.
// Returns the final state of the grid in-case the user wants to save it as an image.
fn simulation_in_background(mut grid: Grid, yx_coordinate_pairs: &Vec<[usize; 2]>) -> Grid {
    let mut rng = thread_rng(); // random number generator

    // Only show the resulting art after its finished rendering (much faster!)
    println!("Running in background");

    loop {
        let mut seen_dead_cell = false;

        for [y, x] in yx_coordinate_pairs {
            let y = *y;
            let x = *x;
            if grid.alive_states[[y, x]] {
                // println!("{} @ {y},{x} is ALIVE", grid.get_cell_on_its_color(y, x));
                grid.spread_to_random_dead_nbor(y, x, &mut rng);
            } else {
                seen_dead_cell = true;
            }
        }
        if !seen_dead_cell {
            return grid;
        }
    }
}

// fn save_vec_as_image(v: &Vec<Vec<[u8; 3]>>, filename: &str) {
//     let height = v.len();
//     let width = v[0].len();

//     // Try using the Image crate to save the image at the desired location
//     let img = image::ImageBuffer::from_fn(
//         width.try_into().unwrap(),
//         height.try_into().unwrap(),
//         |y, x| {
//             let y: usize = y.try_into().unwrap();
//             let x: usize = x.try_into().unwrap();
//             image::Rgb(v[y][x])
//         },
//     );

//     // Print whether saving the image succeeded or not.
//     if let Err(e) = img.save(&format!("output_images/{filename}")) {
//         println!("Sorry, the file wasn't able to because of this error -> {e:?}")
//     } else {
//         println!("{filename} was saved in the output_images directory")
//     }
// }
