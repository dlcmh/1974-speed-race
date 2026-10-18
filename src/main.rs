//! SPEED RACE (Taito, 1974) — pioneer of the vertical scrolling genre.
//! GUI edition: a native window, one dependency (minifb), no GPU framework.
//!
//! The presentation mimics the original black-and-white raster hardware with
//! its colored cellophane screen overlay: white road and blocky sprites on
//! black, tinted red near the top, amber mid-screen, green at the bottom.
//!
//! Controls:
//!   ← / → or A / D   steering wheel
//!   ↑ / W or SPACE   accelerator pedal
//!   ↓ / S            brake
//!   ENTER            start / restart
//!   ESC              quit

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::cmp::max;

// ---------------------------------------------------------------------------
// Screen: native 224x256 (classic vertical arcade raster), scaled 2x by minifb
// ---------------------------------------------------------------------------

const W: usize = 224;
const H: usize = 256;
const SCALE: u32 = 2;

const BLACK: u32 = 0x000000;
const WHITE: u32 = 0xF0F0F0;
const GREY: u32 = 0x909090;
const TRANS: u32 = 0x010101; // sprite transparency key

// Cellophane overlay bands (y ranges on the CRT).
const RED_BAND: u32 = 0xC03020;
const AMBER_BAND: u32 = 0xC09030;
const GREEN_BAND: u32 = 0x30A040;

/// Road band, horizontally centred, like the original's fixed cabinet road.
const ROAD_X0: usize = 40;
const ROAD_X1: usize = W - 40; // exclusive

const GAME_SECONDS: f32 = 90.0;

/// Deterministic pseudo-random in 0..1 (no RNG dependency).
fn hash(n: f32) -> f32 {
    (n.sin() * 43_758.547).fract().abs()
}

#[inline]
fn fmax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
#[inline]
fn fmin(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

struct Screen {
    px: Vec<u32>,
}

impl Screen {
    fn new() -> Screen {
        Screen {
            px: vec![BLACK; W * H],
        }
    }
    fn clear(&mut self) {
        for p in self.px.iter_mut() {
            *p = BLACK;
        }
    }
    #[inline]
    fn set(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && (x as usize) < W && y >= 0 && (y as usize) < H {
            self.px[y as usize * W + x as usize] = c;
        }
    }
    fn rect(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: u32) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x, y, c);
            }
        }
    }
    fn sprite(&mut self, sp: &Sprite, x: i32, y: i32) {
        for (sy, row) in sp.rows.iter().enumerate() {
            for (sx, &p) in row.iter().enumerate() {
                if p != TRANS {
                    self.set(x + sx as i32, y + sy as i32, p);
                }
            }
        }
    }

    /// Cellophane strip overlay: the grey road takes the band color, while
    /// white pixels (markings, cars, text) stay white — matching how the
    /// original's B&W CRT looked under its colored strips.
    fn apply_overlay(&mut self) {
        for y in 0..H {
            let band = if y < 48 {
                RED_BAND
            } else if y < 176 {
                AMBER_BAND
            } else {
                GREEN_BAND
            };
            let r = band >> 16;
            let g = band >> 8;
            let b = band;
            let dim = |c: u32| ((c * 11) / 16) & 0xFF;
            let dimmed = (dim(r) << 16) | (dim(g) << 8) | dim(b);
            for p in self.px[y * W..(y + 1) * W].iter_mut() {
                if *p == BLACK || *p == WHITE {
                    continue;
                }
                *p = if *p == GREY { band } else { dimmed };
            }
        }
    }
}

struct Sprite {
    rows: Vec<Vec<u32>>,
}

impl Sprite {
    /// '#' = white, '.' = transparent, 'o' = grey.
    fn parse(rows: &[&str]) -> Sprite {
        Sprite {
            rows: rows
                .iter()
                .map(|r| {
                    r.chars()
                        .map(|c| match c {
                            '#' => WHITE,
                            'o' => GREY,
                            _ => TRANS,
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sprites: chunky 70s raster cars, 16x24, one pixel per CRT dot
// ---------------------------------------------------------------------------

fn player_car() -> Sprite {
    Sprite::parse(&[
        "................",
        "......####......",
        ".....######.....",
        ".....#oooo#.....",
        ".....#oooo#.....",
        "....########....",
        "...##########...",
        "..############..",
        "..############..",
        ".##############.",
        ".###o######o###.",
        ".###o######o###.",
        "################",
        "################",
        "################",
        "################",
        ".##############.",
        ".###o######o###.",
        ".###o######o###.",
        "..############..",
        "...##########...",
        "....########....",
        ".....######.....",
        "................",
    ])
}

fn rival_car() -> Sprite {
    Sprite::parse(&[
        "................",
        "....########....",
        "...##########...",
        "..############..",
        "..###oooooo###..",
        "..###oooooo###..",
        ".##############.",
        ".##############.",
        ".###o######o###.",
        ".###o######o###.",
        ".##############.",
        ".##############.",
        "..############..",
        "...##########...",
        "....########....",
        "................",
    ])
}

fn truck() -> Sprite {
    Sprite::parse(&[
        "................",
        "..############..",
        "..#oooooooooo#..",
        "..#oooooooooo#..",
        "..#oooooooooo#..",
        "..#oooooooooo#..",
        "..############..",
        "................",
        ".##############.",
        ".##############.",
        ".##############.",
        ".###o######o###.",
        ".##############.",
        ".##############.",
        ".##############.",
        "..############..",
        "...##########...",
        "................",
    ])
}

// ---------------------------------------------------------------------------
// 5x7 arcade font for score / time (each glyph = 7 rows of 5 pixels)
// ---------------------------------------------------------------------------

const FONT: &[(char, &str)] = &[
    ('0', "01110100011001110101110011000101110"),
    ('1', "00100011000010000100001000010001110"),
    ('2', "01110100010000100010001001000011111"),
    ('3', "11111000100010001100000001000101110"),
    ('4', "00010001100101010010111110001000010"),
    ('5', "11111100001111000001000011000101110"),
    ('6', "00110010001000011110100011000101110"),
    ('7', "11111000010001000100010001000100010"),
    ('8', "01110100011000101110100011000101110"),
    ('9', "01110100011000101111000010001011000"),
    ('A', "01110100011000111111100011000110001"),
    ('B', "11110100011000111110100011000111110"),
    ('C', "01110100011000010000100001000101110"),
    ('D', "11100100101000110001100011001011100"),
    ('E', "11111100001000011110100001000011111"),
    ('F', "11111100001000011110100001000010000"),
    ('G', "01110100011000010111100011000101111"),
    ('H', "10001100011000111111100011000110001"),
    ('I', "01110001000010000100001000010001110"),
    ('J', "00111000100001000010000110010001100"),
    ('K', "10001100101010011000101001001010001"),
    ('L', "10000100001000010000100001000011111"),
    ('M', "10001110111010110101100011000110001"),
    ('N', "10001100011100110101110011000110001"),
    ('O', "01110100011000110001100011000101110"),
    ('P', "11110100011000111110100001000010000"),
    ('Q', "01110100011000110001101011001001101"),
    ('R', "11110100011000111110101001001010001"),
    ('S', "01111100001000001110000010000111110"),
    ('T', "11111001000010000100001000010000100"),
    ('U', "10001100011000110001100011000101110"),
    ('V', "10001100011000110001100010101000100"),
    ('W', "10001100011000110101101011101110001"),
    ('X', "10001100010101000100010101000110001"),
    ('Y', "10001100010101000100001000010000100"),
    ('Z', "11111000010001000100010001000011111"),
    ('-', "00000000000000011111000000000000000"),
    ('!', "00100001000010000100001000000000100"),
    (' ', "00000000000000000000000000000000000"),
];

fn draw_text(s: &mut Screen, text: &str, x: i32, y: i32, c: u32) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some((_, pat)) = FONT.iter().find(|(k, _)| *k == ch) {
            for (i, b) in pat.chars().enumerate() {
                if b == '1' {
                    s.set(cx + (i % 5) as i32, y + (i / 5) as i32, c);
                }
            }
        }
        cx += 6;
    }
}

fn text_width(text: &str) -> i32 {
    text.len() as i32 * 6
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

const LANES: [i32; 4] = [64, 96, 128, 160]; // rival lane left-edges

struct Rival {
    x: f32,
    y: f32,
    speed: f32, // world speed as a fraction of the player's
    truck: bool,
    ai_timer: f32,
    cut: f32, // 0 = hold lane, -1/+1 = cutting toward the player
}

struct Game {
    px: f32,    // player x (left edge of sprite)
    speed: f32, // 0.1..1.0 fraction of max
    scroll: i32,
    rivals: Vec<Rival>,
    spawn_timer: f32,
    score: u32,
    dist: u32, // raw distance travelled in pixels; score = dist / 5
    hi: u32,
    time_left: f32,
    crashed: bool,
    crash_timer: f32,
    finished: bool,
}

impl Game {
    fn new(hi: u32) -> Game {
        let mut g = Game {
            px: (ROAD_X0 + ROAD_X1) as f32 / 2.0 - 8.0,
            speed: 0.4,
            scroll: 0,
            rivals: Vec::new(),
            spawn_timer: 0.0,
            score: 0,
            dist: 0,
            hi,
            time_left: GAME_SECONDS,
            crashed: false,
            crash_timer: 0.0,
            finished: false,
        };
        // Starting traffic, one per lane, spaced down the road.
        for (i, &lane) in LANES.iter().enumerate() {
            g.rivals.push(Rival {
                x: lane as f32,
                y: 30.0 + i as f32 * 55.0,
                speed: 0.55 + i as f32 * 0.08,
                truck: i % 3 == 2,
                ai_timer: 0.4 + i as f32 * 0.3,
                cut: 0.0,
            });
        }
        g
    }

    const PLAYER_Y: i32 = H as i32 - 56;

    fn step(&mut self, left: bool, right: bool, accel: bool, brake: bool, dt: f32) {
        if self.finished {
            return;
        }
        self.time_left -= dt;
        if self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.finished = true;
            return;
        }

        if self.crashed {
            self.crash_timer -= dt;
            if self.crash_timer <= 0.0 {
                self.crashed = false;
                self.speed = 0.2;
                self.px = (ROAD_X0 + ROAD_X1) as f32 / 2.0 - 8.0;
                self.rivals.retain(|r| r.y < (H as f32 - 140.0));
            }
            return;
        }

        // Steering.
        let steer = 130.0 * dt * (0.5 + self.speed * 0.8);
        if left && !right {
            self.px -= steer;
        }
        if right && !left {
            self.px += steer;
        }
        // Pedal / brake / engine braking.
        if accel {
            self.speed = fmin(1.0, self.speed + dt * 0.5);
        } else if brake {
            self.speed = fmax(0.1, self.speed - dt * 0.9);
        } else {
            self.speed = fmax(0.1, self.speed - dt * 0.08);
        }

        // Keep the wheels on the tarmac.
        let lo = ROAD_X0 as f32 + 4.0;
        let hi_lim = ROAD_X1 as f32 - 4.0 - 16.0;
        self.px = self.px.clamp(lo, hi_lim);

        // Scroll the world, score on distance.
        let v = 70.0 + self.speed * 290.0;
        let delta = (v * dt) as i32;
        self.scroll += delta;
        self.dist = self.dist.wrapping_add(delta as u32);
        self.score = self.dist / 5;

        // Rivals are slower than the player, so they come toward us. Cars
        // (not trucks) periodically cut toward the player's lane.
        let py = Self::PLAYER_Y as f32;
        let diff = fmin(0.30, self.score as f32 / 50_000.0); // ramps with score
        let prev: Vec<(f32, f32)> = self.rivals.iter().map(|r| (r.x, r.y)).collect();
        for r in self.rivals.iter_mut() {
            r.y += (v * (1.0 - r.speed)) * dt;
            if !r.truck && r.y > -10.0 && r.y < py - 20.0 {
                r.ai_timer -= dt;
                if r.ai_timer <= 0.0 {
                    r.ai_timer = 0.5 + hash(r.x * 0.17 + self.scroll as f32 * 0.013) * 1.2;
                    if hash(r.y * 0.23 + self.scroll as f32 * 0.019) < 0.6 + diff {
                        r.cut = (self.px - r.x).signum();
                    }
                }
                if r.cut != 0.0 {
                    // Give up the cut once alongside the player.
                    if (r.x - self.px).abs() < 3.0 || r.y > py - 15.0 {
                        r.cut = 0.0;
                    } else {
                        r.x += r.cut * (40.0 + diff * 60.0) * dt;
                    }
                }
            }
            // Stay on the tarmac.
            r.x = r
                .x
                .clamp((ROAD_X0 + 4) as f32, (ROAD_X1 - 4 - 16) as f32);
        }
        // Polite traffic: nobody drives into anybody. A rival whose move
        // landed it inside another one is put back where it was and drops
        // its lane cut (eagerness denied).
        let half_w = 16.0; // sprite width
        let body_h = |r: &Rival| -> f32 { if r.truck { 17.0 } else { 15.0 } };
        let overlapping: Vec<usize> = (0..self.rivals.len())
            .filter(|&i| {
                self.rivals.iter().enumerate().any(|(j, rj)| {
                    i != j
                        && (self.rivals[i].x - rj.x).abs() < half_w
                        && (self.rivals[i].y - rj.y).abs()
                            < body_h(&self.rivals[i]).max(body_h(rj))
                })
            })
            .collect();
        for &i in &overlapping {
            let (ox, oy) = prev[i];
            let ri = &mut self.rivals[i];
            ri.x = ox;
            ri.y = oy;
            ri.cut = 0.0;
        }
        // Traffic AI: match speed of the vehicle just ahead (near lane).
        for i in 0..self.rivals.len() {
            for j in 0..self.rivals.len() {
                if i != j
                    && (self.rivals[j].x - self.rivals[i].x).abs() < half_w
                    && self.rivals[j].y < self.rivals[i].y
                    && self.rivals[i].y - self.rivals[j].y < 90.0
                    && self.rivals[i].speed > self.rivals[j].speed
                {
                    self.rivals[i].speed = self.rivals[j].speed;
                }
            }
        }
        self.rivals.retain(|r| r.y < (H + 30) as f32);

        // Spawn traffic at the top of the road. Traffic thickens with score.
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.spawn_timer = fmax(0.45, (0.9 - self.speed * 0.5) - diff * 0.5);
            let lane = LANES[(self.scroll / 2137) as usize % LANES.len()];
            let blocked = self
                .rivals
                .iter()
                .any(|r| (r.x - lane as f32).abs() < 12.0 && r.y < 60.0);
            if !blocked {
                let hs = self.scroll as f32;
                self.rivals.push(Rival {
                    x: lane as f32,
                    y: -30.0,
                    speed: 0.45 + (hash(hs * 0.011) * 0.3) + diff * 0.6,
                    truck: hash(hs * 0.007) < 0.25,
                    ai_timer: 0.8 + hash(hs * 0.029) * 1.5,
                    cut: 0.0,
                });
            }
        }

        // Collision (box test with a couple of pixels of forgiveness).
        let (pw, ph) = (14, 22);
        let (hx, hy) = (self.px as i32 + 1, Self::PLAYER_Y + 1);
        let hit = self.rivals.iter().any(|r| {
            let (rw, rh) = if r.truck { (14, 16) } else { (14, 14) };
            hx < r.x as i32 + rw
                && hx + pw > r.x as i32
                && hy < r.y as i32 + rh
                && hy + ph > r.y as i32
        });
        if hit {
            self.crashed = true;
            self.crash_timer = 1.5;
            self.time_left -= 5.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn draw_world(s: &mut Screen, g: &Game, t: f32) {
    s.clear();

    // Road surface: the original drew the road as a solid light band.
    s.rect(ROAD_X0 as i32, 0, (ROAD_X1 - ROAD_X0) as i32, H as i32, GREY);

    // Road edge rails (white), dashed by the scroll. The world moves DOWN
    // the screen as scroll grows, so the phase is (y - scroll).
    let scr = g.scroll;
    for y in 0..H as i32 {
        let phase = ((y - scr) % 16 + 16) % 16;
        if phase < 10 {
            s.set(ROAD_X0 as i32 + 1, y, WHITE);
            s.set(ROAD_X1 as i32 - 2, y, WHITE);
        }
    }
    // Three dashed lane dividers.
    for &lx in &[LANES[1] - 8, LANES[2] - 8, LANES[3] - 8] {
        for y in 0..H as i32 {
            let phase = ((y - scr) % 32 + 32) % 32;
            if phase < 18 {
                s.set(lx, y, WHITE);
            }
        }
    }

    // Rival traffic.
    let car = rival_car();
    let tk = truck();
    for r in &g.rivals {
        let sp = if r.truck { &tk } else { &car };
        s.sprite(sp, r.x as i32, r.y as i32);
    }

    // Player car (blank-flashes after a crash, like the original's blink).
    let visible = !g.crashed || (t * 8.0) as i32 % 2 == 0;
    if visible {
        let pc = player_car();
        s.sprite(&pc, g.px as i32, Game::PLAYER_Y);
    }

    // Top HUD strip: TIME on the left, score/hi in the arcade font.
    s.rect(0, 0, W as i32, 22, BLACK);
    let secs = max(0, g.time_left.ceil() as i32);
    draw_text(s, "TIME", 8, 4, WHITE);
    draw_text(s, &format!("{:02}", secs), 34, 4, WHITE);
    draw_text(s, &format!("1UP {}", g.score * 10), 88, 4, WHITE);
    draw_text(s, &format!("HI {}", g.hi * 10), 160, 4, WHITE);

    // Crash flash: invert-ish white burst at the crash site.
    if g.crashed && (t * 16.0) as i32 % 2 == 0 {
        s.rect(g.px as i32 - 4, Game::PLAYER_Y - 4, 24, 32, WHITE);
    }

    // The cellophane overlay bands that made the B&W CRT look colored.
    s.apply_overlay();
}

fn attract_screen(hi: u32, blink: bool) -> Vec<u32> {
    let mut s = Screen::new();
    // A road cruising behind the panel.
    let mut demo = Game::new(hi);
    demo.speed = 0.6;
    for _ in 0..300 {
        demo.step(false, false, true, false, 1.0 / 60.0);
    }
    demo.finished = false;
    draw_world(&mut s, &demo, 0.0);

    // Dark panel with the marquee.
    s.rect(20, 88, (W - 40) as i32, 84, BLACK);
    s.rect(20, 88, (W - 40) as i32, 2, WHITE);
    s.rect(20, 170, (W - 40) as i32, 2, WHITE);
    let title = "SPEED RACE";
    draw_text(
        &mut s,
        title,
        (W as i32 - text_width(title)) / 2,
        100,
        WHITE,
    );
    let sub = "TAITO 1974";
    draw_text(
        &mut s,
        sub,
        (W as i32 - text_width(sub)) / 2,
        116,
        WHITE,
    );
    if blink {
        let msg = "PRESS ENTER";
        draw_text(
            &mut s,
            msg,
            (W as i32 - text_width(msg)) / 2,
            140,
            WHITE,
        );
    }
    s.px
}

fn game_over_screen(g: &Game, blink: bool) -> Vec<u32> {
    let mut s = Screen::new();
    draw_world(&mut s, g, 0.0);
    s.rect(20, 96, (W - 40) as i32, 70, BLACK);
    s.rect(20, 96, (W - 40) as i32, 2, WHITE);
    s.rect(20, 164, (W - 40) as i32, 2, WHITE);
    let record = g.score >= g.hi && g.score > 0;
    let l1 = if record { "NEW HI SCORE!" } else { "TIME UP" };
    draw_text(&mut s, l1, (W as i32 - text_width(l1)) / 2, 108, WHITE);
    let l2 = format!("SCORE {}", g.score * 10);
    draw_text(&mut s, &l2, (W as i32 - text_width(&l2)) / 2, 124, WHITE);
    if blink {
        let l3 = "PRESS ENTER";
        draw_text(&mut s, l3, (W as i32 - text_width(l3)) / 2, 144, WHITE);
    }
    s.px
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let win_w = W * SCALE as usize;
    let win_h = H * SCALE as usize;
    let mut window = Window::new(
        "SPEED RACE — Taito 1974 (Rust)",
        win_w,
        win_h,
        WindowOptions {
            scale: minifb::Scale::X2,
            resize: false,
            ..WindowOptions::default()
        },
    )
    .expect("unable to open window");
    window.set_target_fps(60);

    let mut hi: u32 = std::fs::read_to_string("highscore.txt")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    let mut state = if std::env::var_os("SR_AUTOSTART").is_some() {
        1 // debug: skip the attract screen
    } else {
        0
    };
    let quit_after = std::env::var("SR_QUIT_SECS")
        .ok()
        .and_then(|s| s.parse::<f32>().ok());
    let mut game = Game::new(hi);
    let mut t = 0f32;
    let mut blink_t = 0f32;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = 1.0 / 60.0;
        t += dt;
        blink_t += dt;
        if let Some(secs) = quit_after
            && t > secs {
                break;
            }

        let left = window.is_key_down(Key::Left) || window.is_key_down(Key::A);
        let right = window.is_key_down(Key::Right) || window.is_key_down(Key::D);
        let accel = window.is_key_down(Key::Up)
            || window.is_key_down(Key::W)
            || window.is_key_down(Key::Space);
        let brake = window.is_key_down(Key::Down) || window.is_key_down(Key::S);
        let start = window.is_key_pressed(Key::Enter, KeyRepeat::No);

        let buf = match state {
            0 => {
                if start {
                    game = Game::new(hi);
                    state = 1;
                }
                attract_screen(hi, (blink_t * 2.0) as i32 % 2 == 0)
            }
            1 => {
                game.step(left, right, accel, brake, dt);
                if game.finished {
                    hi = max(hi, game.score);
                    let _ = std::fs::write("highscore.txt", hi.to_string());
                    state = 2;
                    blink_t = 0.0;
                }
                let mut s = Screen::new();
                draw_world(&mut s, &game, t);
                s.px
            }
            _ => {
                if start {
                    state = 0;
                    blink_t = 0.0;
                }
                game_over_screen(&game, (blink_t * 1.5) as i32 % 2 == 0)
            }
        };

        window.update_with_buffer(&buf, W, H).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lane markings must move DOWN the screen as the world scrolls forward
    /// (a sign error made them crawl upward).
    #[test]
    fn road_markings_scroll_downward() {
        let mut base = Game::new(0);
        base.speed = 0.0;
        let mut s0 = Screen::new();
        draw_world(&mut s0, &base, 0.0);
        let lx = LANES[1] - 8;
        // Measure dash STARTS (white run onsets) below the 22px HUD strip,
        // where the pattern is unobscured.
        let first_onset = |s: &Screen| -> i32 {
            for y in 40..H as i32 - 1 {
                let here = s.px[y as usize * W + lx as usize] == WHITE;
                let above = s.px[(y - 1) as usize * W + lx as usize] == WHITE;
                if here && !above {
                    return y;
                }
            }
            -1
        };
        let y0 = first_onset(&s0);
        assert!(y0 > 0, "no lane dash onset found at scroll 0");
        base.scroll = 8; // world advances 8 px => dashes shift DOWN 8 px
        let mut s1 = Screen::new();
        draw_world(&mut s1, &base, 0.0);
        let y1 = first_onset(&s1);
        assert_eq!(
            (y1 - y0 + 32) % 32,
            8,
            "dashes moved up instead of down (onset {} -> {})",
            y0, y1
        );
    }

    /// Score must tick up continuously, not sit frozen (integer-truncation bug).
    #[test]
    fn score_ticks_every_second() {
        let mut g = Game::new(0);
        let mut last = 0u32;
        for f in 0..1800 {
            g.step(false, false, true, false, 1.0 / 60.0);
            if (f + 1) % 180 == 0 {
                assert!(
                    g.score > last,
                    "score frozen at {} for over 3 seconds (frame {})",
                    last, f
                );
                last = g.score;
            }
        }
    }

    /// Rivals must never overlap each other, however eager they get.
    #[test]
    fn rivals_never_overlap() {
        let mut g = Game::new(0);
        let body_h = |r: &Rival| -> f32 { if r.truck { 17.0 } else { 15.0 } };
        for f in 0..36_000 {
            g.time_left = GAME_SECONDS;
            g.step(false, false, true, false, 1.0 / 60.0);
            g.finished = false;
            for i in 0..g.rivals.len() {
                for j in (i + 1)..g.rivals.len() {
                    let (a, b) = (&g.rivals[i], &g.rivals[j]);
                    let bad = (a.x - b.x).abs() < 15.5
                        && (a.y - b.y).abs() < body_h(a).max(body_h(b));
                    assert!(
                        !bad,
                        "rivals overlap at frame {}: ({:.1},{:.1}) vs ({:.1},{:.1})",
                        f, a.x, a.y, b.x, b.y
                    );
                }
            }
        }
    }

    /// Rivals must not stay welded to their spawn lane.
    #[test]
    fn rivals_change_lanes() {
        let mut g = Game::new(0);
        let mut max_offlane = 0.0f32;
        for _ in 0..60_000 {
            g.time_left = GAME_SECONDS; // ignore the time limit for this sim
            g.step(false, false, true, false, 1.0 / 60.0);
            g.finished = false;
            for r in &g.rivals {
                let off = LANES
                    .iter()
                    .map(|&l| (r.x - l as f32).abs())
                    .fold(f32::INFINITY, fmin);
                max_offlane = fmax(max_offlane, off);
            }
        }
        assert!(
            max_offlane > 10.0,
            "rivals never left their lane (max offlane {})",
            max_offlane
        );
    }
}
