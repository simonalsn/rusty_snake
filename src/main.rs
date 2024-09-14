use piston_window::*;
use rand::Rng;
use find_folder;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

const BLOCK_SIZE: f64 = 25.0;
const WIDTH: i32 = 30;
const HEIGHT: i32 = 20;
const SNAKE_SPEED: u64 = 15;
const HIGH_SCORE_FILE: &str = "high_scores.txt";
const MAX_HIGH_SCORES: usize = 5;

#[derive(Clone, PartialEq)]
enum Direction {
    Right,
    Left,
    Up,
    Down,
}

#[derive(Clone, PartialEq)]
enum FoodType {
    RustyScrap,
    ShinyMetal,
    Water,
}

#[derive(Clone, PartialEq)]
enum SegmentType {
    Head,
    Tail,
    EmptyStomach,
    FullStomach,
}

struct Food {
    position: (i32, i32),
    food_type: FoodType,
}

struct Segment {
    position: (i32, i32),
    segment_type: SegmentType,
}

struct Snake {
    body: Vec<Segment>,
    direction: Direction,
}

struct HighScoreEntry {
    name: String,
    score: u32,
}

struct Game {
    snake: Snake,
    foods: Vec<Food>,
    score: u32,
    game_over: bool,
    game_started: bool,
    frame_count: u64,
    wrap_around: bool,
    tail_length: usize, // Keeps track of tail growth
    high_scores: Vec<HighScoreEntry>,
    entering_name: bool,
    player_name: String,
}

impl Game {
    fn new() -> Game {
        let mut snake_body = Vec::new();
        let head_pos = (WIDTH / 2, HEIGHT / 2);
        snake_body.push(Segment {
            position: head_pos,
            segment_type: SegmentType::Head,
        });

        let mut game = Game {
            snake: Snake {
                body: snake_body,
                direction: Direction::Right,
            },
            foods: Vec::new(),
            score: 0,
            game_over: false,
            game_started: false,
            frame_count: 0,
            wrap_around: true,
            tail_length: 0, // Tail starts at length 0
            high_scores: Vec::new(),
            entering_name: false,
            player_name: String::new(),
        };
        game.load_high_scores();
        game
    }

    fn spawn_foods(&mut self) {
        self.foods.clear();
        self.foods.push(self.generate_food(FoodType::RustyScrap));
        self.foods.push(self.generate_food(FoodType::ShinyMetal));
        self.foods.push(self.generate_food(FoodType::Water));
    }

    fn generate_food(&self, food_type: FoodType) -> Food {
        let mut rng = rand::thread_rng();
        loop {
            let position = (rng.gen_range(0..WIDTH), rng.gen_range(0..HEIGHT));
            if !self.snake.body.iter().any(|seg| seg.position == position)
                && !self.foods.iter().any(|f| f.position == position)
            {
                return Food { position, food_type };
            }
        }
    }

    fn update(&mut self) {
        self.frame_count += 1;

        if self.game_over || !self.game_started || self.frame_count % SNAKE_SPEED != 0 {
            return;
        }

        // Spawn foods if not already present
        if self.foods.is_empty() {
            self.spawn_foods();
        }

        // Calculate new head position
        let head_segment = &self.snake.body[0];
        let (head_x, head_y) = head_segment.position;
        let new_head_pos = match self.snake.direction {
            Direction::Right => (head_x + 1, head_y),
            Direction::Left => (head_x - 1, head_y),
            Direction::Up => (head_x, head_y - 1),
            Direction::Down => (head_x, head_y + 1),
        };

        let new_head_pos = if self.wrap_around {
            ((new_head_pos.0 + WIDTH) % WIDTH, (new_head_pos.1 + HEIGHT) % HEIGHT)
        } else if new_head_pos.0 < 0
            || new_head_pos.0 >= WIDTH
            || new_head_pos.1 < 0
            || new_head_pos.1 >= HEIGHT
        {
            self.game_over = true;
            self.check_high_score();
            return;
        } else {
            new_head_pos
        };

        // Check for collision with self
        if self.snake.body.iter().any(|seg| seg.position == new_head_pos) {
            self.game_over = true;
            self.check_high_score();
            return;
        }

        // Check for food at new head position
        let mut ate_food = false;
        let mut food_type = None;
        if let Some(index) = self.foods.iter().position(|food| food.position == new_head_pos) {
            ate_food = true;
            food_type = Some(self.foods[index].food_type.clone());
            self.foods[index] = self.generate_food(self.foods[index].food_type.clone());
        }

        // Move segments
        let mut new_positions: Vec<(i32, i32)> = vec![new_head_pos];
        for i in 0..self.snake.body.len() - 1 {
            new_positions.push(self.snake.body[i].position);
        }
        for (segment, &new_pos) in self.snake.body.iter_mut().zip(new_positions.iter()) {
            segment.position = new_pos;
        }

        // Update segment types if necessary
        // Ensure the first segment is always the head
        self.snake.body[0].segment_type = SegmentType::Head;

        // Handle food effects
        if ate_food {
            match food_type.unwrap() {
                FoodType::RustyScrap => {
                    self.score += 1;
                    if self.tail_length < 3 {
                        // Growing the tail
                        self.tail_length += 1;
                        let tail_pos = self.snake.body.last().unwrap().position;
                        self.snake.body.push(Segment {
                            position: tail_pos,
                            segment_type: SegmentType::Tail,
                        });
                    } else {
                        // After tail is fully grown, add empty stomach segments between head and tail
                        let stomach_insert_index = 1; // After head
                        let stomach_pos = self.snake.body[stomach_insert_index - 1].position;
                        self.snake.body.insert(
                            stomach_insert_index,
                            Segment {
                                position: stomach_pos,
                                segment_type: SegmentType::EmptyStomach,
                            },
                        );
                    }
                }
                FoodType::ShinyMetal => {
                    // Check if snake length >= 5 (head + tail of 3 + at least one stomach segment)
                    if self.snake.body.len() < 5 {
                        self.game_over = true;
                        self.check_high_score();
                        return;
                    }
                    // Check for empty stomach segment
                    if let Some(empty_stomach_index) = self
                        .snake
                        .body
                        .iter()
                        .position(|seg| seg.segment_type == SegmentType::EmptyStomach)
                    {
                        // Change one empty stomach segment to full stomach
                        self.snake.body[empty_stomach_index].segment_type = SegmentType::FullStomach;
                        self.score += 2;
                    } else {
                        // No empty stomach segments, game over
                        self.game_over = true;
                        self.check_high_score();
                        return;
                    }
                }
                FoodType::Water => {
                    // Check if there is any full stomach segment
                    if let Some(full_stomach_index) = self
                        .snake
                        .body
                        .iter()
                        .position(|seg| seg.segment_type == SegmentType::FullStomach)
                    {
                        // Change one full stomach segment back to empty stomach
                        self.snake.body[full_stomach_index].segment_type = SegmentType::EmptyStomach;
                        self.score += 5;
                        // Grow tail by adding empty stomach segments before the tail
                        let tail_start_index = self
                            .snake
                            .body
                            .iter()
                            .position(|seg| seg.segment_type == SegmentType::Tail)
                            .unwrap();
                        let tail_pos = self.snake.body[tail_start_index].position;
                        for _ in 0..5 {
                            self.snake.body.insert(
                                tail_start_index,
                                Segment {
                                    position: tail_pos,
                                    segment_type: SegmentType::EmptyStomach,
                                },
                            );
                        }
                    } else {
                        // No shiny scrap stored, do nothing
                        // As per your request
                    }
                }
            }
        }
    }

    fn check_high_score(&mut self) {
        if self.is_high_score() {
            self.entering_name = true;
            self.player_name.clear();
        }
    }

    fn load_high_scores(&mut self) {
        // Try to open the high score file
        if let Ok(file) = File::open(HIGH_SCORE_FILE) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(entry) = line {
                    let parts: Vec<&str> = entry.split(',').collect();
                    if parts.len() == 2 {
                        if let Ok(score) = parts[1].parse::<u32>() {
                            self.high_scores.push(HighScoreEntry {
                                name: parts[0].to_string(),
                                score,
                            });
                        }
                    }
                }
            }
            // Sort high scores in descending order
            self.high_scores.sort_by(|a, b| b.score.cmp(&a.score));
            // Keep only top N scores
            self.high_scores.truncate(MAX_HIGH_SCORES);
        }
    }

    fn save_high_scores(&self) {
        if let Ok(mut file) = File::create(HIGH_SCORE_FILE) {
            for entry in &self.high_scores {
                if let Err(e) = writeln!(file, "{},{}", entry.name, entry.score) {
                    eprintln!("Error writing high scores: {}", e);
                    break;
                }
            }
        } else {
            eprintln!("Error creating high score file.");
        }
    }

    fn is_high_score(&self) -> bool {
        if self.high_scores.len() < MAX_HIGH_SCORES {
            return true;
        }
        self.score > self.high_scores.last().unwrap().score
    }

    fn add_high_score(&mut self) {
        self.high_scores.push(HighScoreEntry {
            name: self.player_name.clone(),
            score: self.score,
        });
        // Sort and truncate
        self.high_scores.sort_by(|a, b| b.score.cmp(&a.score));
        self.high_scores.truncate(MAX_HIGH_SCORES);
        // Save to file
        self.save_high_scores();
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new(
        "Rusty Snake",
        [(WIDTH as f64) * BLOCK_SIZE, (HEIGHT as f64) * BLOCK_SIZE],
    )
    .exit_on_esc(true)
    .build()
    .unwrap();

    // Load the font for displaying text
    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let font_path = assets.join("FiraSans-Regular.ttf");
    let mut glyphs = match window.load_font(&font_path) {
        Ok(g) => g,
        Err(_) => {
            eprintln!("Error: Font file 'FiraSans-Regular.ttf' not found in 'assets' folder.");
            std::process::exit(1);
        }
    };

    let mut game = Game::new();

    while let Some(event) = window.next() {
        if let Some(Button::Keyboard(key)) = event.press_args() {
            if game.game_over {
                if game.entering_name {
                    match key {
                        Key::Return => {
                            if !game.player_name.is_empty() {
                                game.add_high_score();
                                game.entering_name = false;
                            }
                        }
                        Key::Backspace => {
                            game.player_name.pop();
                        }
                        _ => {
                            if let Some(c) = key_to_char(key) {
                                if game.player_name.len() < 10 {
                                    game.player_name.push(c);
                                }
                            }
                        }
                    }
                } else {
                    if key == Key::Return {
                        game = Game::new(); // Restart the game
                    }
                }
            } else if !game.game_started {
                match key {
                    Key::Right | Key::Left | Key::Up | Key::Down => {
                        game.game_started = true;
                        game.snake.direction = match key {
                            Key::Right => Direction::Right,
                            Key::Left => Direction::Left,
                            Key::Up => Direction::Up,
                            Key::Down => Direction::Down,
                            _ => unreachable!(),
                        };
                        game.spawn_foods();
                    }
                    _ => {}
                }
            } else {
                match key {
                    Key::Right if game.snake.direction != Direction::Left => {
                        game.snake.direction = Direction::Right
                    }
                    Key::Left if game.snake.direction != Direction::Right => {
                        game.snake.direction = Direction::Left
                    }
                    Key::Up if game.snake.direction != Direction::Down => {
                        game.snake.direction = Direction::Up
                    }
                    Key::Down if game.snake.direction != Direction::Up => {
                        game.snake.direction = Direction::Down
                    }
                    _ => {}
                }
            }
        }

        window.draw_2d(&event, |c, g, device| {
            clear([0.5, 0.5, 0.5, 1.0], g);

            if game.game_over {
                if game.entering_name {
                    // Display 'Enter Your Name'
                    let transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 180.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 - 20.0,
                    );
                    text::Text::new_color([1.0, 1.0, 1.0, 1.0], 24)
                        .draw(
                            "New High Score! Enter Your Name:",
                            &mut glyphs,
                            &c.draw_state,
                            transform,
                            g,
                        )
                        .unwrap();

                    // Display player name being entered
                    let name_transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 50.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 + 20.0,
                    );
                    text::Text::new_color([0.0, 1.0, 0.0, 1.0], 32)
                        .draw(&game.player_name, &mut glyphs, &c.draw_state, name_transform, g)
                        .unwrap();
                } else {
                    // Display 'Game Over' and the final score
                    let transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 80.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 - 100.0,
                    );
                    text::Text::new_color([1.0, 0.0, 0.0, 1.0], 32)
                        .draw("Game Over", &mut glyphs, &c.draw_state, transform, g)
                        .unwrap();

                    let score_transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 90.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 - 60.0,
                    );
                    text::Text::new_color([1.0, 1.0, 1.0, 1.0], 24)
                        .draw(
                            &format!("Final Score: {}", game.score),
                            &mut glyphs,
                            &c.draw_state,
                            score_transform,
                            g,
                        )
                        .unwrap();

                    // Display High Scores
                    let hs_title_transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 70.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 - 20.0,
                    );
                    text::Text::new_color([1.0, 0.8, 0.0, 1.0], 28)
                        .draw("High Scores", &mut glyphs, &c.draw_state, hs_title_transform, g)
                        .unwrap();

                    for (i, entry) in game.high_scores.iter().enumerate() {
                        let hs_transform = c.transform.trans(
                            (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 100.0,
                            (HEIGHT as f64 * BLOCK_SIZE) / 2.0 + (i as f64 * 30.0),
                        );
                        text::Text::new_color([1.0, 1.0, 1.0, 1.0], 24)
                            .draw(
                                &format!("{}: {} - {}", i + 1, entry.name, entry.score),
                                &mut glyphs,
                                &c.draw_state,
                                hs_transform,
                                g,
                            )
                            .unwrap();
                    }

                    let restart_transform = c.transform.trans(
                        (WIDTH as f64 * BLOCK_SIZE) / 2.0 - 120.0,
                        (HEIGHT as f64 * BLOCK_SIZE) / 2.0 + 200.0,
                    );
                    text::Text::new_color([1.0, 1.0, 1.0, 1.0], 20)
                        .draw(
                            "Press Enter to Restart",
                            &mut glyphs,
                            &c.draw_state,
                            restart_transform,
                            g,
                        )
                        .unwrap();
                }
            } else if !game.game_started {
                let flash = (game.frame_count as f64 / 30.0).sin() * 0.5 + 0.5;
                rectangle(
                    [0.0, 0.0, 1.0, flash as f32],
                    [
                        0.0,
                        0.0,
                        (WIDTH as f64) * BLOCK_SIZE,
                        (HEIGHT as f64) * BLOCK_SIZE,
                    ],
                    c.transform,
                    g,
                );

                // Display 'Press Arrow Key to Start'
                let transform = c.transform.trans(50.0, (HEIGHT as f64 * BLOCK_SIZE) / 2.0);
                text::Text::new_color([1.0, 1.0, 1.0, flash as f32], 24)
                    .draw(
                        "Press Arrow Key to Start",
                        &mut glyphs,
                        &c.draw_state,
                        transform,
                        g,
                    )
                    .unwrap();
            } else {
                // Draw snake
                for segment in &game.snake.body {
                    let (x, y) = segment.position;
                    let (size, color) = match segment.segment_type {
                        SegmentType::Head => (BLOCK_SIZE, [0.0, 0.7, 0.0, 1.0]), // Dark green for head
                        SegmentType::FullStomach => (BLOCK_SIZE, [0.0, 1.0, 0.0, 1.0]), // Bright green for full stomach
                        SegmentType::EmptyStomach => (20.0, [0.0, 0.8, 0.0, 1.0]), // Medium green for empty stomach
                        SegmentType::Tail => (15.0, [0.0, 0.5, 0.0, 1.0]), // Darker green for tail
                    };

                    // Center the smaller segments within the grid cell
                    let rect_x = x as f64 * BLOCK_SIZE + (BLOCK_SIZE - size) / 2.0;
                    let rect_y = y as f64 * BLOCK_SIZE + (BLOCK_SIZE - size) / 2.0;

                    rectangle(
                        color,
                        [rect_x, rect_y, size, size],
                        c.transform,
                        g,
                    );
                }

                // Draw food
                for food in &game.foods {
                    let color = match food.food_type {
                        FoodType::RustyScrap => [0.6, 0.4, 0.2, 1.0], // Brown
                        FoodType::ShinyMetal => [0.8, 0.8, 0.8, 1.0], // Silver
                        FoodType::Water => [0.0, 0.0, 1.0, 1.0],       // Blue
                    };
                    rectangle(
                        color,
                        [
                            food.position.0 as f64 * BLOCK_SIZE,
                            food.position.1 as f64 * BLOCK_SIZE,
                            BLOCK_SIZE,
                            BLOCK_SIZE,
                        ],
                        c.transform,
                        g,
                    );
                }

                // Draw score
                let score_transform = c.transform.trans(10.0, 20.0);
                text::Text::new_color([1.0, 1.0, 1.0, 1.0], 20)
                    .draw(
                        &format!("Score: {}", game.score),
                        &mut glyphs,
                        &c.draw_state,
                        score_transform,
                        g,
                    )
                    .unwrap();
            }

            // Update glyphs
            glyphs.factory.encoder.flush(device);
        });

        event.update(|_| {
            game.update();
        });
    }
}

// Helper function to convert Key to char
fn key_to_char(key: Key) -> Option<char> {
    match key {
        Key::A => Some('A'),
        Key::B => Some('B'),
        Key::C => Some('C'),
        Key::D => Some('D'),
        Key::E => Some('E'),
        Key::F => Some('F'),
        Key::G => Some('G'),
        Key::H => Some('H'),
        Key::I => Some('I'),
        Key::J => Some('J'),
        Key::K => Some('K'),
        Key::L => Some('L'),
        Key::M => Some('M'),
        Key::N => Some('N'),
        Key::O => Some('O'),
        Key::P => Some('P'),
        Key::Q => Some('Q'),
        Key::R => Some('R'),
        Key::S => Some('S'),
        Key::T => Some('T'),
        Key::U => Some('U'),
        Key::V => Some('V'),
        Key::W => Some('W'),
        Key::X => Some('X'),
        Key::Y => Some('Y'),
        Key::Z => Some('Z'),
        Key::Space => Some(' '),
        _ => None,
    }
}
