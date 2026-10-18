use macroquad::prelude::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
mod score;

fn window_conf() -> Conf {
    Conf {
        window_title: "My Fullscreen Game".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

const DEFAULT_BINDINGS: [KeyCode; 6] = [
    KeyCode::A,
    KeyCode::D,
    KeyCode::W,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Up,
];
const BINDING_ROWS: usize = 6;
const SETTINGS_ROWS: usize = BINDING_ROWS + 1;
const CONTROLS_FILE: &str = "keybinds.cfg";
const SPEED: f32 = 300.0;
const DOWN_SPEED: f32 = 500.0;
const PLAYER_COLOR: Color = GREEN;
const HAZARD_COLOR: Color = RED;
const SAFE_COLOR: Color = GRAY;
const RADIUS: f32 = 30.0;

const OBSTACLE_SPEED: f32 = 200.0;
const SPAWN_INTERVAL: f32 = 0.2;
const CONTACT_EPS: f32 = 2.0;
const BOUNDS_EPS: f32 = 0.5;
const HAZARD_ROLL: i32 = 3;

const CIRCLE_MIN_R: f32 = 20.0;
const CIRCLE_MAX_R: f32 = 45.0;
const RECT_MIN_W: f32 = 60.0;
const RECT_MAX_W: f32 = 160.0;
const RECT_MIN_H: f32 = 25.0;
const RECT_MAX_H: f32 = 60.0;

struct Object {
    x: f32,
    y: f32,
    vy: f32,
    red: bool,
}

struct Circle {
    obj: Object,
    radius: f32,
}

struct Rectangle {
    obj: Object,
    width: f32,
    height: f32,
}

enum Obstacle {
    Circ(Circle),
    Rect(Rectangle),
}

enum Push {
    None,
    To(f32, f32),
    Crushed,
}

impl Obstacle {
    fn vy(&self) -> f32 {
        match self {
            Obstacle::Circ(c) => c.obj.vy,
            Obstacle::Rect(r) => r.obj.vy,
        }
    }

    fn is_red(&self) -> bool {
        match self {
            Obstacle::Circ(c) => c.obj.red,
            Obstacle::Rect(r) => r.obj.red,
        }
    }

    fn update(&mut self, dt: f32) {
        match self {
            Obstacle::Circ(c) => c.obj.y += c.obj.vy * dt,
            Obstacle::Rect(r) => r.obj.y += r.obj.vy * dt,
        }
    }

    fn bounds(&self) -> (f32, f32, f32, f32) {
        match self {
            Obstacle::Circ(c) => (
                c.obj.x - c.radius,
                c.obj.y - c.radius,
                c.obj.x + c.radius,
                c.obj.y + c.radius,
            ),
            Obstacle::Rect(r) => (
                r.obj.x - r.width / 2.0,
                r.obj.y - r.height / 2.0,
                r.obj.x + r.width / 2.0,
                r.obj.y + r.height / 2.0,
            ),
        }
    }

    fn draw(&self) {
        let color = if self.is_red() {
            HAZARD_COLOR
        } else {
            SAFE_COLOR
        };
        match self {
            Obstacle::Circ(c) => draw_circle(c.obj.x, c.obj.y, c.radius, color),
            Obstacle::Rect(r) => draw_rectangle(
                r.obj.x - r.width / 2.0,
                r.obj.y - r.height / 2.0,
                r.width,
                r.height,
                color,
            ),
        }
    }

    fn overlaps_circle(&self, px: f32, py: f32, pr: f32) -> bool {
        match self {
            Obstacle::Circ(c) => {
                let dx = px - c.obj.x;
                let dy = py - c.obj.y;
                let reach = c.radius + pr;
                dx * dx + dy * dy < reach * reach
            }
            Obstacle::Rect(_) => {
                let (left, top, right, bottom) = self.bounds();
                let closest_x = px.clamp(left, right);
                let closest_y = py.clamp(top, bottom);
                let dx = px - closest_x;
                let dy = py - closest_y;
                dx * dx + dy * dy < pr * pr
            }
        }
    }

    fn push_cy(&self, px: f32, py: f32, pr: f32, screen_h: f32) -> Push {
        if !self.overlaps_circle(px, py, pr) {
            return Push::None;
        }
        let (target_cy, sign) = match self {
            Obstacle::Circ(c) => {
                let dx = px - c.obj.x;
                let reach = c.radius + pr;
                let dy = (reach * reach - dx * dx).max(0.0).sqrt();
                if py <= c.obj.y {
                    (c.obj.y - dy, -1.0)
                } else {
                    (c.obj.y + dy, 1.0)
                }
            }
            Obstacle::Rect(r) => {
                let (_, top, _, bottom) = self.bounds();
                if py <= r.obj.y {
                    (top - pr, -1.0)
                } else {
                    (bottom + pr, 1.0)
                }
            }
        };
        if target_cy - pr < 0.0 || target_cy + pr > screen_h {
            Push::Crushed
        } else {
            Push::To(target_cy, sign)
        }
    }

    fn offscreen(&self, screen_h: f32) -> bool {
        let (_, top, _, bottom) = self.bounds();
        if self.vy() < 0.0 {
            bottom < 0.0
        } else {
            top > screen_h
        }
    }
}

struct Player {
    x: f32,
    y: f32,
    radius: f32,
    grav: Gravity,
    moving: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gravity {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Menu,
    Settings,
    Playing,
    GameOver,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    OnePlayer,
    TwoPlayer,
}

const ALL_KEYCODES: &[KeyCode] = &[
    KeyCode::Space,
    KeyCode::Apostrophe,
    KeyCode::Comma,
    KeyCode::Minus,
    KeyCode::Period,
    KeyCode::Slash,
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
    KeyCode::Semicolon,
    KeyCode::Equal,
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
    KeyCode::LeftBracket,
    KeyCode::Backslash,
    KeyCode::RightBracket,
    KeyCode::GraveAccent,
    KeyCode::World1,
    KeyCode::World2,
    KeyCode::Escape,
    KeyCode::Enter,
    KeyCode::Tab,
    KeyCode::Backspace,
    KeyCode::Insert,
    KeyCode::Delete,
    KeyCode::Right,
    KeyCode::Left,
    KeyCode::Down,
    KeyCode::Up,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::CapsLock,
    KeyCode::ScrollLock,
    KeyCode::NumLock,
    KeyCode::PrintScreen,
    KeyCode::Pause,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
    KeyCode::F13,
    KeyCode::F14,
    KeyCode::F15,
    KeyCode::F16,
    KeyCode::F17,
    KeyCode::F18,
    KeyCode::F19,
    KeyCode::F20,
    KeyCode::F21,
    KeyCode::F22,
    KeyCode::F23,
    KeyCode::F24,
    KeyCode::F25,
    KeyCode::Kp0,
    KeyCode::Kp1,
    KeyCode::Kp2,
    KeyCode::Kp3,
    KeyCode::Kp4,
    KeyCode::Kp5,
    KeyCode::Kp6,
    KeyCode::Kp7,
    KeyCode::Kp8,
    KeyCode::Kp9,
    KeyCode::KpDecimal,
    KeyCode::KpDivide,
    KeyCode::KpMultiply,
    KeyCode::KpSubtract,
    KeyCode::KpAdd,
    KeyCode::KpEnter,
    KeyCode::KpEqual,
    KeyCode::LeftShift,
    KeyCode::LeftControl,
    KeyCode::LeftAlt,
    KeyCode::LeftSuper,
    KeyCode::RightShift,
    KeyCode::RightControl,
    KeyCode::RightAlt,
    KeyCode::RightSuper,
    KeyCode::Menu,
    KeyCode::Back,
    KeyCode::Unknown,
];

fn keycode_name(key: KeyCode) -> String {
    format!("{key:?}")
}

fn keycode_from_name(name: &str) -> Option<KeyCode> {
    ALL_KEYCODES
        .iter()
        .copied()
        .find(|k| keycode_name(*k) == name)
}

#[derive(Clone, Copy)]
struct Controls {
    bindings: [KeyCode; 6],
}

impl Controls {
    fn new() -> Self {
        Controls {
            bindings: DEFAULT_BINDINGS,
        }
    }

    fn left(&self, player: usize) -> KeyCode {
        self.bindings[player * 3]
    }

    fn right(&self, player: usize) -> KeyCode {
        self.bindings[player * 3 + 1]
    }

    fn toggle(&self, player: usize) -> KeyCode {
        self.bindings[player * 3 + 2]
    }

    fn set(&mut self, index: usize, key: KeyCode) {
        self.bindings[index] = key;
    }

    fn reset(&mut self) {
        self.bindings = DEFAULT_BINDINGS;
    }

    fn to_text(self) -> String {
        self.bindings
            .iter()
            .map(|k| keycode_name(*k))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn from_text(text: &str) -> Self {
        let mut controls = Controls::new();
        for (index, line) in text.lines().enumerate() {
            if index >= controls.bindings.len() {
                break;
            }
            if let Some(key) = keycode_from_name(line.trim()) {
                controls.bindings[index] = key;
            }
        }
        controls
    }
}

fn save_controls(controls: &Controls) {
    let _ = fs::write(CONTROLS_FILE, controls.to_text());
}

fn load_controls() -> Controls {
    fs::read_to_string(CONTROLS_FILE)
        .map(|text| Controls::from_text(&text))
        .unwrap_or_else(|_| Controls::new())
}

impl Player {
    fn new(x: f32, y: f32, grav: Gravity) -> Self {
        Player {
            x,
            y,
            radius: RADIUS,
            grav,
            moving: false,
        }
    }

    fn travel_sign(&self) -> f32 {
        match self.grav {
            Gravity::Up => -1.0,
            Gravity::Down => 1.0,
        }
    }

    fn toggle_gravity(&mut self) {
        self.grav = match self.grav {
            Gravity::Up => Gravity::Down,
            Gravity::Down => Gravity::Up,
        };
    }
}

fn swap_move(player: &mut Player, sizes: &[f32; 2], radius: &f32, speed: &f32) {
    match player.grav {
        Gravity::Up => {
            let limit: f32 = *radius;
            player.y -= speed * get_frame_time();
            if player.y <= limit {
                player.y = limit;
                player.moving = false;
            }
        }
        Gravity::Down => {
            let limit: f32 = sizes[1] - radius;
            player.y += speed * get_frame_time();
            if player.y >= limit {
                player.y = limit;
                player.moving = false;
            }
        }
    }
}

fn spawn_obstacle(sizes: &[f32; 2], from_bottom: bool) -> Obstacle {
    let red = rand::gen_range(0, HAZARD_ROLL) == 0;
    let vy = if from_bottom {
        -OBSTACLE_SPEED
    } else {
        OBSTACLE_SPEED
    };
    if rand::gen_range(0, 2) == 0 {
        let radius = rand::gen_range(CIRCLE_MIN_R, CIRCLE_MAX_R);
        let x = rand::gen_range(radius, sizes[0] - radius);
        let y = if from_bottom {
            sizes[1] + radius
        } else {
            -radius
        };
        Obstacle::Circ(Circle {
            obj: Object { x, y, vy, red },
            radius,
        })
    } else {
        let width = rand::gen_range(RECT_MIN_W, RECT_MAX_W);
        let height = rand::gen_range(RECT_MIN_H, RECT_MAX_H);
        let half_w = width / 2.0;
        let x = rand::gen_range(half_w, sizes[0] - half_w);
        let y = if from_bottom {
            sizes[1] + height / 2.0
        } else {
            -height / 2.0
        };
        Obstacle::Rect(Rectangle {
            obj: Object { x, y, vy, red },
            width,
            height,
        })
    }
}

fn move_horizontal(player: &mut Player, dx: f32, obstacles: &[Obstacle], sizes: &[f32; 2]) {
    let new_x = (player.x + dx).clamp(player.radius, sizes[0] - player.radius);
    let blocked = obstacles
        .iter()
        .any(|o| !o.is_red() && o.overlaps_circle(new_x, player.y, player.radius));
    if !blocked {
        player.x = new_x;
    }
}

fn gravity_side_contact(player: &Player, obstacle: &Obstacle) -> bool {
    match obstacle {
        Obstacle::Circ(c) => {
            let correct_side = match player.grav {
                Gravity::Down => player.y <= c.obj.y,
                Gravity::Up => player.y >= c.obj.y,
            };
            if !correct_side {
                return false;
            }
            let dx = player.x - c.obj.x;
            let dy = player.y - c.obj.y;
            let reach = c.radius + player.radius;
            ((dx * dx + dy * dy).sqrt() - reach).abs() <= CONTACT_EPS
        }
        Obstacle::Rect(r) => {
            let (left, top, right, bottom) = obstacle.bounds();
            if player.x < left || player.x > right {
                return false;
            }
            match player.grav {
                Gravity::Down => {
                    player.y <= r.obj.y && (player.y + player.radius - top).abs() <= CONTACT_EPS
                }
                Gravity::Up => {
                    player.y >= r.obj.y && (player.y - player.radius - bottom).abs() <= CONTACT_EPS
                }
            }
        }
    }
}

fn support_vy(player: &Player, obstacles: &[Obstacle]) -> Option<f32> {
    obstacles
        .iter()
        .find(|o| !o.is_red() && gravity_side_contact(player, o))
        .map(|o| o.vy())
}

fn hits_red(player: &Player, obstacles: &[Obstacle]) -> bool {
    obstacles
        .iter()
        .any(|o| o.is_red() && o.overlaps_circle(player.x, player.y, player.radius))
}

fn resolve_obstacles(player: &mut Player, obstacles: &[Obstacle], sizes: &[f32; 2]) -> bool {
    for _ in 0..4 {
        let mut pushed = false;
        for obstacle in obstacles {
            if obstacle.is_red() {
                continue;
            }
            match obstacle.push_cy(player.x, player.y, player.radius, sizes[1]) {
                Push::None => {}
                Push::Crushed => return true,
                Push::To(cy, sign) => {
                    player.y = cy;
                    if player.moving && sign != player.travel_sign() {
                        player.moving = false;
                    }
                    pushed = true;
                }
            }
        }
        if !pushed {
            break;
        }
    }
    obstacles.iter().any(|o| {
        !o.is_red() && o.overlaps_circle(player.x, player.y, (player.radius - 1.0).max(0.0))
    })
}

fn out_of_bounds(player: &Player, sizes: &[f32; 2]) -> bool {
    player.y - player.radius < -BOUNDS_EPS || player.y + player.radius > sizes[1] + BOUNDS_EPS
}

fn at_gravity_limit(player: &Player, sizes: &[f32; 2]) -> bool {
    match player.grav {
        Gravity::Up => player.y <= player.radius + BOUNDS_EPS,
        Gravity::Down => player.y >= sizes[1] - player.radius - BOUNDS_EPS,
    }
}

fn is_supported(player: &Player, obstacles: &[Obstacle]) -> bool {
    obstacles
        .iter()
        .any(|o| !o.is_red() && gravity_side_contact(player, o))
}

fn should_fall(player: &Player, obstacles: &[Obstacle], sizes: &[f32; 2]) -> bool {
    !at_gravity_limit(player, sizes) && !is_supported(player, obstacles)
}

fn start_players(mode: Mode, sizes: &[f32; 2]) -> Vec<Player> {
    match mode {
        Mode::OnePlayer => vec![Player::new(sizes[0] / 2.0, RADIUS, Gravity::Up)],
        Mode::TwoPlayer => vec![
            Player::new(sizes[0] / 2.0, RADIUS, Gravity::Up),
            Player::new(sizes[0] / 2.0, sizes[1] - RADIUS, Gravity::Down),
        ],
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );

    let sizes: [f32; 2] = [screen_width(), screen_height()];

    let mut controls = load_controls();
    let mut mode = Mode::TwoPlayer;
    let mut players: Vec<Player> = Vec::new();
    let mut obstacles: Vec<Obstacle> = Vec::new();
    let mut spawn_timer = 0.0_f32;
    let mut from_bottom = true;
    let mut state = GameState::Menu;
    let mut settings_sel: usize = 0;
    let mut listening = false;
    let mut top_scores: Vec<score::ScoreEntry> = Vec::new();
    let mut scores_rx = Some(score::fetch_top_scores());
    let mut survival = 0.0_f32;
    loop {
        if let Some(rx) = &scores_rx {
            if let Ok(list) = rx.try_recv() {
                top_scores = list;
                scores_rx = None;
            }
        }

        if state == GameState::Playing {
            let dt = get_frame_time();
            survival += dt;
            for (i, player) in players.iter_mut().enumerate() {
                let mut dx = 0.0;
                if is_key_down(controls.left(i)) {
                    dx -= SPEED * dt;
                }
                if is_key_down(controls.right(i)) {
                    dx += SPEED * dt;
                }
                move_horizontal(player, dx, &obstacles, &sizes);
            }

            for (i, player) in players.iter_mut().enumerate() {
                if is_key_pressed(controls.toggle(i)) && !player.moving {
                    player.toggle_gravity();
                    player.moving = true;
                }
            }

            spawn_timer += dt;
            if spawn_timer >= SPAWN_INTERVAL {
                spawn_timer -= SPAWN_INTERVAL;
                obstacles.push(spawn_obstacle(&sizes, from_bottom));
                from_bottom = !from_bottom;
            }

            let supports: Vec<Option<f32>> =
                players.iter().map(|p| support_vy(p, &obstacles)).collect();
            for obstacle in obstacles.iter_mut() {
                obstacle.update(dt);
            }
            for (player, support) in players.iter_mut().zip(supports) {
                if let Some(vy) = support {
                    player.y += vy * dt;
                }
            }

            for player in players.iter_mut() {
                if player.moving {
                    swap_move(player, &sizes, &RADIUS, &DOWN_SPEED);
                }
            }

            let mut dead = players.iter().any(|p| hits_red(p, &obstacles));
            if !dead {
                for player in players.iter_mut() {
                    dead |= resolve_obstacles(player, &obstacles, &sizes);
                }
            }
            if !dead {
                dead = players.iter().any(|p| out_of_bounds(p, &sizes));
            }
            if !dead {
                dead = players.iter().any(|p| hits_red(p, &obstacles));
            }

            if !dead {
                for player in players.iter_mut() {
                    if should_fall(player, &obstacles, &sizes) {
                        player.moving = true;
                    }
                }
            }

            for player in players.iter_mut() {
                player.x = player.x.clamp(RADIUS, sizes[0] - RADIUS);
            }
            obstacles.retain(|o| !o.offscreen(sizes[1]));

            if dead {
                state = GameState::GameOver;
                score::submit_score("Ilam".to_string(), survival as u32);
            }
        } else if state == GameState::Menu {
            if is_key_pressed(KeyCode::Key1) {
                mode = Mode::OnePlayer;
                state = GameState::Playing;
            } else if is_key_pressed(KeyCode::Key2) {
                mode = Mode::TwoPlayer;
                state = GameState::Playing;
            } else if is_key_pressed(KeyCode::S) {
                settings_sel = 0;
                listening = false;
                state = GameState::Settings;
            }
            if state == GameState::Playing {
                obstacles.clear();
                spawn_timer = 0.0;
                survival = 0.0;
                from_bottom = true;
                players = start_players(mode, &sizes);
            }
        } else if state == GameState::Settings {
            if listening {
                if let Some(key) = get_last_key_pressed() {
                    if key == KeyCode::Escape {
                        listening = false;
                    } else {
                        controls.set(settings_sel, key);
                        save_controls(&controls);
                        listening = false;
                    }
                }
            } else {
                if is_key_pressed(KeyCode::Down) {
                    settings_sel = (settings_sel + 1) % SETTINGS_ROWS;
                }
                if is_key_pressed(KeyCode::Up) {
                    settings_sel = (settings_sel + SETTINGS_ROWS - 1) % SETTINGS_ROWS;
                }
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                    if settings_sel == BINDING_ROWS {
                        controls.reset();
                        save_controls(&controls);
                    } else {
                        listening = true;
                    }
                }
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::Menu;
                }
            }
        } else if state == GameState::GameOver && is_key_pressed(KeyCode::R) {
            obstacles.clear();
            spawn_timer = 0.0;
            survival = 0.0;
            from_bottom = true;
            players = start_players(mode, &sizes);
            state = GameState::Playing;
        } else if state == GameState::GameOver && is_key_pressed(KeyCode::M) {
            obstacles.clear();
            players.clear();
            scores_rx = Some(score::fetch_top_scores());
            state = GameState::Menu;
        }

        clear_background(BLACK);
        for obstacle in &obstacles {
            obstacle.draw();
        }
        for player in &players {
            draw_circle(player.x, player.y, player.radius, PLAYER_COLOR);
        }

        if state == GameState::Menu {
            let title = "GRAVITY DODGE";
            let title_dims = measure_text(title, None, 70, 1.0);
            draw_text(
                title,
                (sizes[0] - title_dims.width) / 2.0,
                sizes[1] / 2.0 - 80.0,
                70.0,
                WHITE,
            );
            let one = "1 - One Player";
            let one_dims = measure_text(one, None, 40, 1.0);
            draw_text(
                one,
                (sizes[0] - one_dims.width) / 2.0,
                sizes[1] / 2.0 + 20.0,
                40.0,
                PLAYER_COLOR,
            );
            let two = "2 - Two Players";
            let two_dims = measure_text(two, None, 40, 1.0);
            draw_text(
                two,
                (sizes[0] - two_dims.width) / 2.0,
                sizes[1] / 2.0 + 80.0,
                40.0,
                PLAYER_COLOR,
            );
            let settings = "S - Settings";
            let settings_dims = measure_text(settings, None, 40, 1.0);
            draw_text(
                settings,
                (sizes[0] - settings_dims.width) / 2.0,
                sizes[1] / 2.0 + 140.0,
                40.0,
                PLAYER_COLOR,
            );
            for (i, e) in top_scores.iter().enumerate() {
                draw_text(
                    &format!("{}. {} - {}", i + 1, e.name, e.score),
                    20.0,
                    40.0 + i as f32 * 30.0,
                    28.0,
                    WHITE,
                );
            }
        }

        if state == GameState::Settings {
            let title = "SETTINGS";
            let title_dims = measure_text(title, None, 60, 1.0);
            draw_text(
                title,
                (sizes[0] - title_dims.width) / 2.0,
                sizes[1] / 2.0 - 210.0,
                60.0,
                WHITE,
            );
            let row_h = 48.0;
            let start_y = sizes[1] / 2.0 - 130.0;
            for row in 0..SETTINGS_ROWS {
                let selected = row == settings_sel;
                let label = if row == BINDING_ROWS {
                    "Reset to Defaults".to_string()
                } else {
                    let player = row / 3 + 1;
                    let slot = ["Left", "Right", "Toggle"][row % 3];
                    format!("Player {player} {slot}")
                };
                let value = if row == BINDING_ROWS {
                    String::new()
                } else if listening && selected {
                    "Press a key...".to_string()
                } else {
                    keycode_name(controls.bindings[row])
                };
                let color = if selected { YELLOW } else { WHITE };
                let label_dims = measure_text(&label, None, 36, 1.0);
                let x = (sizes[0] - label_dims.width) / 2.0 - 180.0;
                let y = start_y + row as f32 * row_h;
                draw_text(&label, x, y, 36.0, color);
                if !value.is_empty() {
                    draw_text(&value, x + 420.0, y, 36.0, color);
                }
            }
            let hint = if listening {
                "Press a new key... (Esc to cancel)"
            } else {
                "Up/Down select    Enter rebind    Esc back"
            };
            let hint_dims = measure_text(hint, None, 30, 1.0);
            draw_text(
                hint,
                (sizes[0] - hint_dims.width) / 2.0,
                start_y + SETTINGS_ROWS as f32 * row_h + 40.0,
                30.0,
                PLAYER_COLOR,
            );
        }

        if state == GameState::GameOver {
            let title = "GAME OVER";
            let title_dims = measure_text(title, None, 80, 1.0);
            draw_text(
                title,
                (sizes[0] - title_dims.width) / 2.0,
                (sizes[1] - title_dims.height) / 2.0,
                80.0,
                HAZARD_COLOR,
            );
            let hint = "Press R to restart    M for menu";
            let hint_dims = measure_text(hint, None, 36, 1.0);
            draw_text(
                hint,
                (sizes[0] - hint_dims.width) / 2.0,
                (sizes[1] + hint_dims.height) / 2.0 + 40.0,
                36.0,
                WHITE,
            );
        }

        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32, red: bool, vy: f32) -> Obstacle {
        Obstacle::Rect(Rectangle {
            obj: Object { x, y, vy, red },
            width: w,
            height: h,
        })
    }

    fn circ(x: f32, y: f32, r: f32, red: bool, vy: f32) -> Obstacle {
        Obstacle::Circ(Circle {
            obj: Object { x, y, vy, red },
            radius: r,
        })
    }

    #[test]
    fn rect_overlap_detects_circle() {
        let o = rect(100.0, 100.0, 40.0, 40.0, false, 10.0);
        assert!(o.overlaps_circle(100.0, 100.0, 30.0));
        assert!(!o.overlaps_circle(100.0, 40.0, 30.0));
    }

    #[test]
    fn circle_overlap_detects_circle() {
        let o = circ(100.0, 100.0, 20.0, false, 10.0);
        assert!(o.overlaps_circle(100.0, 140.0, 30.0));
        assert!(!o.overlaps_circle(100.0, 155.0, 30.0));
    }

    #[test]
    fn rect_push_target_and_room() {
        let o = rect(100.0, 100.0, 40.0, 40.0, false, 10.0);
        match o.push_cy(100.0, 95.0, 30.0, 600.0) {
            Push::To(cy, sign) => {
                assert!((cy - 50.0).abs() < 1e-3);
                assert!((sign - (-1.0)).abs() < 1e-6);
            }
            _ => panic!("expected a push to the top surface"),
        }
    }

    #[test]
    fn push_into_edge_is_crushed() {
        let o = rect(100.0, 40.0, 40.0, 40.0, false, 10.0);
        assert!(matches!(o.push_cy(100.0, 30.0, 30.0, 600.0), Push::Crushed));
    }

    #[test]
    fn touching_is_not_a_push() {
        let o = rect(100.0, 100.0, 40.0, 40.0, false, 10.0);
        assert!(matches!(o.push_cy(100.0, 50.0, 30.0, 600.0), Push::None));
    }

    #[test]
    fn circle_push_lands_on_curved_surface() {
        let o = circ(100.0, 100.0, 20.0, false, -10.0);
        match o.push_cy(100.0, 80.0, 30.0, 600.0) {
            Push::To(cy, sign) => {
                assert!((cy - 50.0).abs() < 1e-3);
                assert!((sign - (-1.0)).abs() < 1e-6);
            }
            _ => panic!("expected a push onto the circle surface"),
        }
    }

    #[test]
    fn standing_on_rect_is_supported() {
        let o = rect(100.0, 100.0, 40.0, 40.0, false, 10.0);
        assert!(gravity_side_contact(
            &Player::new(100.0, 50.0, Gravity::Down),
            &o
        ));
        assert!(!gravity_side_contact(
            &Player::new(100.0, 40.0, Gravity::Down),
            &o
        ));
        assert!(!gravity_side_contact(
            &Player::new(100.0, 50.0, Gravity::Up),
            &o
        ));
    }

    #[test]
    fn standing_on_circle_is_supported() {
        let o = circ(100.0, 100.0, 20.0, false, -50.0);
        assert!(gravity_side_contact(
            &Player::new(100.0, 50.0, Gravity::Down),
            &o
        ));
        assert!(!gravity_side_contact(
            &Player::new(100.0, 40.0, Gravity::Down),
            &o
        ));
    }

    #[test]
    fn gravity_toggles() {
        let mut player = Player::new(100.0, 50.0, Gravity::Up);
        player.toggle_gravity();
        assert_eq!(player.grav, Gravity::Down);
        player.toggle_gravity();
        assert_eq!(player.grav, Gravity::Up);
    }

    #[test]
    fn despawns_past_the_far_edge() {
        assert!(rect(100.0, -100.0, 40.0, 40.0, false, -100.0).offscreen(600.0));
        assert!(rect(100.0, 700.0, 40.0, 40.0, false, 100.0).offscreen(600.0));
        assert!(!rect(100.0, 300.0, 40.0, 40.0, false, 100.0).offscreen(600.0));
    }

    #[test]
    fn at_gravity_limit_only_at_the_gravity_side_edge() {
        let sizes = [800.0, 600.0];
        assert!(at_gravity_limit(
            &Player::new(100.0, RADIUS, Gravity::Up),
            &sizes
        ));
        assert!(at_gravity_limit(
            &Player::new(100.0, 600.0 - RADIUS, Gravity::Down),
            &sizes
        ));
        assert!(!at_gravity_limit(
            &Player::new(100.0, 300.0, Gravity::Up),
            &sizes
        ));
        assert!(!at_gravity_limit(
            &Player::new(100.0, 300.0, Gravity::Down),
            &sizes
        ));
        assert!(!at_gravity_limit(
            &Player::new(100.0, RADIUS, Gravity::Down),
            &sizes
        ));
    }

    #[test]
    fn unsupported_mid_screen_falls() {
        let sizes = [800.0, 600.0];
        let player = Player::new(100.0, 300.0, Gravity::Down);
        assert!(should_fall(&player, &[], &sizes));
    }

    #[test]
    fn resting_on_platform_does_not_fall() {
        let sizes = [800.0, 600.0];
        let platform = rect(100.0, 300.0, 40.0, 40.0, false, 0.0);
        let player = Player::new(100.0, 280.0 - RADIUS, Gravity::Down);
        assert!(!should_fall(&player, &[platform], &sizes));
    }

    #[test]
    fn resting_at_edge_does_not_fall() {
        let sizes = [800.0, 600.0];
        let player = Player::new(100.0, RADIUS, Gravity::Up);
        assert!(!should_fall(&player, &[], &sizes));
    }

    #[test]
    fn frozen_mid_air_after_push_recovers() {
        let sizes = [800.0, 600.0];
        let mut player = Player::new(100.0, 300.0, Gravity::Down);
        player.moving = false;
        if should_fall(&player, &[], &sizes) {
            player.moving = true;
        }
        assert!(player.moving);
    }

    #[test]
    fn one_player_mode_starts_one_player() {
        let sizes = [800.0, 600.0];
        let players = start_players(Mode::OnePlayer, &sizes);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].grav, Gravity::Up);
        assert_eq!(players[0].y, RADIUS);
    }

    #[test]
    fn two_player_mode_starts_two_players() {
        let sizes = [800.0, 600.0];
        let players = start_players(Mode::TwoPlayer, &sizes);
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].grav, Gravity::Up);
        assert_eq!(players[0].y, RADIUS);
        assert_eq!(players[1].grav, Gravity::Down);
        assert_eq!(players[1].y, sizes[1] - RADIUS);
    }

    #[test]
    fn default_controls_match_original_scheme() {
        let controls = Controls::new();
        assert_eq!(controls.left(0), KeyCode::A);
        assert_eq!(controls.right(0), KeyCode::D);
        assert_eq!(controls.toggle(0), KeyCode::W);
        assert_eq!(controls.left(1), KeyCode::Left);
        assert_eq!(controls.right(1), KeyCode::Right);
        assert_eq!(controls.toggle(1), KeyCode::Up);
    }

    #[test]
    fn set_updates_the_selected_slot() {
        let mut controls = Controls::new();
        controls.set(1, KeyCode::J);
        assert_eq!(controls.right(0), KeyCode::J);
        assert_eq!(controls.left(0), KeyCode::A);
        controls.set(5, KeyCode::Slash);
        assert_eq!(controls.toggle(1), KeyCode::Slash);
    }

    #[test]
    fn reset_restores_defaults() {
        let mut controls = Controls::new();
        controls.set(0, KeyCode::Z);
        controls.set(4, KeyCode::Backspace);
        controls.reset();
        assert_eq!(controls.bindings, DEFAULT_BINDINGS);
    }

    #[test]
    fn text_round_trips() {
        let mut controls = Controls::new();
        controls.set(0, KeyCode::Comma);
        controls.set(3, KeyCode::F5);
        let restored = Controls::from_text(&controls.to_text());
        assert_eq!(restored.bindings, controls.bindings);
    }

    #[test]
    fn from_text_keeps_defaults_for_missing_or_invalid_lines() {
        let restored = Controls::from_text("Q\nnotakey\n\nUp");
        assert_eq!(restored.bindings[0], KeyCode::Q);
        assert_eq!(restored.bindings[1], DEFAULT_BINDINGS[1]);
        assert_eq!(restored.bindings[2], DEFAULT_BINDINGS[2]);
        assert_eq!(restored.bindings[3], KeyCode::Up);
        assert_eq!(restored.bindings[5], DEFAULT_BINDINGS[5]);
    }

    #[test]
    fn keycode_name_round_trips() {
        for key in ALL_KEYCODES {
            assert_eq!(keycode_from_name(&keycode_name(*key)), Some(*key));
        }
    }
}
