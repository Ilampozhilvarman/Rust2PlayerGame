use macroquad::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn window_conf() -> Conf {
    Conf {
        window_title: "My Fullscreen Game".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

const PLAYER_SWAP_KEYS: [KeyCode; 2] = [KeyCode::W, KeyCode::Up];
const PLAYER_MOVE_KEYS: [KeyCode; 4] = [KeyCode::A, KeyCode::D, KeyCode::Left, KeyCode::Right];
const SPEED: f32 = 300.0;
const DOWN_SPEED: f32 = 500.0;
const PLAYER_COLOR: Color = GREEN;
const HAZARD_COLOR: Color = RED;
const SAFE_COLOR: Color = GRAY;
const RADIUS: f32 = 30.0;

const OBSTACLE_SPEED: f32 = 200.0;
const SPAWN_INTERVAL: f32 = 1.0;
const CONTACT_EPS: f32 = 2.0;
const BOUNDS_EPS: f32 = 0.5;
const HAZARD_ROLL: i32 = 5;

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
    supported: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gravity {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Playing,
    GameOver,
}

impl Player {
    fn new(x: f32, y: f32, grav: Gravity) -> Self {
        Player {
            x,
            y,
            radius: RADIUS,
            grav,
            moving: false,
            supported: false,
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

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );

    let sizes: [f32; 2] = [screen_width(), screen_height()];

    let mut player1 = Player::new(sizes[0] / 2.0, RADIUS, Gravity::Up);
    let mut player2 = Player::new(sizes[0] / 2.0, sizes[1] - RADIUS, Gravity::Down);
    let mut obstacles: Vec<Obstacle> = Vec::new();
    let mut spawn_timer = 0.0_f32;
    let mut from_bottom = true;
    let mut state = GameState::Playing;

    loop {
        if state == GameState::Playing {
            let dt = get_frame_time();

            let mut dx1 = 0.0;
            if is_key_down(PLAYER_MOVE_KEYS[0]) {
                dx1 -= SPEED * dt;
            }
            if is_key_down(PLAYER_MOVE_KEYS[1]) {
                dx1 += SPEED * dt;
            }
            move_horizontal(&mut player1, dx1, &obstacles, &sizes);

            let mut dx2 = 0.0;
            if is_key_down(PLAYER_MOVE_KEYS[2]) {
                dx2 -= SPEED * dt;
            }
            if is_key_down(PLAYER_MOVE_KEYS[3]) {
                dx2 += SPEED * dt;
            }
            move_horizontal(&mut player2, dx2, &obstacles, &sizes);

            if is_key_pressed(PLAYER_SWAP_KEYS[0]) && !player1.moving {
                player1.toggle_gravity();
                player1.moving = true;
            }
            if is_key_pressed(PLAYER_SWAP_KEYS[1]) && !player2.moving {
                player2.toggle_gravity();
                player2.moving = true;
            }

            spawn_timer += dt;
            if spawn_timer >= SPAWN_INTERVAL {
                spawn_timer -= SPAWN_INTERVAL;
                obstacles.push(spawn_obstacle(&sizes, from_bottom));
                from_bottom = !from_bottom;
            }

            let support1 = support_vy(&player1, &obstacles);
            let support2 = support_vy(&player2, &obstacles);
            for obstacle in obstacles.iter_mut() {
                obstacle.update(dt);
            }
            if let Some(vy) = support1 {
                player1.y += vy * dt;
            }
            if let Some(vy) = support2 {
                player2.y += vy * dt;
            }

            if player1.moving {
                swap_move(&mut player1, &sizes, &RADIUS, &DOWN_SPEED);
            }
            if player2.moving {
                swap_move(&mut player2, &sizes, &RADIUS, &DOWN_SPEED);
            }

            let mut dead = hits_red(&player1, &obstacles) || hits_red(&player2, &obstacles);
            if !dead {
                dead = resolve_obstacles(&mut player1, &obstacles, &sizes)
                    || resolve_obstacles(&mut player2, &obstacles, &sizes);
            }
            if !dead {
                dead = out_of_bounds(&player1, &sizes) || out_of_bounds(&player2, &sizes);
            }
            if !dead {
                dead = hits_red(&player1, &obstacles) || hits_red(&player2, &obstacles);
            }

            let now1 = obstacles
                .iter()
                .any(|o| !o.is_red() && gravity_side_contact(&player1, o));
            if player1.supported && !now1 && !dead {
                player1.moving = true;
            }
            player1.supported = now1;

            let now2 = obstacles
                .iter()
                .any(|o| !o.is_red() && gravity_side_contact(&player2, o));
            if player2.supported && !now2 && !dead {
                player2.moving = true;
            }
            player2.supported = now2;

            player1.x = player1.x.clamp(RADIUS, sizes[0] - RADIUS);
            player2.x = player2.x.clamp(RADIUS, sizes[0] - RADIUS);
            obstacles.retain(|o| !o.offscreen(sizes[1]));

            if dead {
                state = GameState::GameOver;
            }
        } else if is_key_pressed(KeyCode::R) {
            obstacles.clear();
            spawn_timer = 0.0;
            from_bottom = true;
            player1 = Player::new(sizes[0] / 2.0, RADIUS, Gravity::Up);
            player2 = Player::new(sizes[0] / 2.0, sizes[1] - RADIUS, Gravity::Down);
            state = GameState::Playing;
        }

        clear_background(BLACK);
        for obstacle in &obstacles {
            obstacle.draw();
        }
        draw_circle(player1.x, player1.y, player1.radius, PLAYER_COLOR);
        draw_circle(player2.x, player2.y, player2.radius, PLAYER_COLOR);

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
            let hint = "Press R to restart";
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
}
