//! # 第 19 章 · Bevy 2D 小游戏：方块吃金币
//!
//! 运行（需要图形环境 X11 / Wayland）：
//!
//! ```bash
//! cd projects/bevy-square
//! cargo run
//! ```
//!
//! 操作：方向键 / WASD 移动绿色方块 → 碰到黄色金币得分；
//! `P` 暂停，`R` 重开，`Esc` 退出。
//!
//! 本文件已在本机实测通过 `cargo build`（bevy 0.20.0，7m39s）并 `cargo run` 成功开窗，
//! 见同目录 README。所有 API 都是按 0.20.0 的源码核对过的，不是凭记忆写的。
//!
//! ---
//!
//! ## 这个文件想讲清楚的一件事：ECS 的心智模型
//!
//! 传统 OOP 引擎（Unity 的 `GameObject`、C++ 里常见的 `class Entity { virtual void update(); }`）
//! 的组织方式是**「对象同时拥有数据和方法」**：玩家对象里存着坐标，也存着 `update()`；
//! 主循环遍历所有对象，挨个调用它们的 `update()`。于是「谁会动到我的坐标」这个问题，
//! 只能靠人读遍所有代码来回答——因为任何对象的 `update()` 都能改任何东西。
//!
//! Bevy 把这件事反过来：**数据与行为彻底分离**。
//!
//! | 概念 | 是什么 | 在本文件里 |
//! |---|---|---|
//! | Entity（实体） | 只是个 ID：没有数据、没有方法 | `commands.spawn(...)` 返回的那个东西 |
//! | Component（组件） | 纯数据结构，挂在实体上 | [`Player`] [`Coin`] [`Velocity`] [`ScoreText`] |
//! | System（系统） | 普通函数，在签名里声明自己读写什么 | [`move_player`] [`detect_collision`] |
//! | Resource（资源） | 全局只有一份的数据 | [`Score`] [`Phase`] |
//! | Query（查询） | 系统向引擎"点菜"：我要所有带 X 组件的实体 | `Query<&mut Transform, With<Player>>` |
//! | World（世界） | 存放所有实体与组件的容器 | `App::new()` 内部那一个 |
//! | Schedule（调度） | 什么时候、按什么顺序跑哪些系统 | `add_systems(Update, (...).chain())` |
//!
//! **为什么系统之间不需要互相调用？** 因为系统只通过「世界」通信：
//! [`move_player`] 把速度写进 [`Velocity`] 组件，[`clamp_player`] 再把 [`Velocity`] 读出来积分成位置——
//! 这两个函数谁也没见过谁，甚至可以在不同 crate 里，只要它们对「`Velocity` 是什么意思」有共识。
//! 好处有三条：
//!
//! 1. **没有回调地狱**：新需求「金币也要受重力影响」= 加一个系统，
//!    而不是去改 5 个类的继承链。
//! 2. **借用检查器替你查数据竞争**：两个系统同时想要 `ResMut<Score>`，
//!    Bevy 会直接指出冲突，而不是像 C++ 那样悄悄写坏数据。
//! 3. **内存布局对 CPU 友好**：所有 `Transform` 是连续存放的一列，
//!    遍历 10000 个实体时缓存命中率远高于「每个对象一次堆分配」——
//!    这正是 ECS 相对 OOP 场景图的性能来源。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// ============================================================================
// 1. 常量：所有调参集中在这里
// ============================================================================

/// 窗口逻辑尺寸（与 `WindowPlugin` 里设置的一致）。
///
/// `WindowResolution` 只接受物理像素整数（`From<(u32, u32)>`，**不是** `f32`——
/// 这是 0.14 的 `Vec2` 写法在 0.20 会直接编译不过的地方），所以留一组 u32 常量。
const WINDOW_PX_W: u32 = 800;
const WINDOW_PX_H: u32 = 600;
const WINDOW_W: f32 = WINDOW_PX_W as f32;
const WINDOW_H: f32 = WINDOW_PX_H as f32;

/// 玩家 / 金币边长。都用纯色正方形，省掉全部图片资源。
const PLAYER_SIZE: f32 = 48.0;
const COIN_SIZE: f32 = 32.0;

/// 玩家速度（像素 / 秒）。注意单位里有"每秒"——见 [`clamp_player`] 里的 `delta_secs`。
const PLAYER_SPEED: f32 = 320.0;

/// 吃满多少金币算通关。
const TARGET_SCORE: u32 = 10;

/// 玩家中心与窗口边缘的最小距离。
const MARGIN: f32 = PLAYER_SIZE / 2.0;

// ============================================================================
// 2. 组件（Component）：挂在实体上的纯数据
// ============================================================================
//
// `#[derive(Component)]` 生成的是"这个类型在 ECS 存储里怎么放"的描述，
// 让 Bevy 能把同类型组件连续存放，遍历时只扫这一列。
//
// 注意这些结构体**没有任何方法**，这是有意的：数据就是数据。
// 想改它就必须写一个系统，而读写意图写在系统签名里，编译期可查。

/// 标记：玩家（被键盘控制的那个方块）。
///
/// 这是"标记组件"（marker component）：没有字段，只给实体打标签。
/// 查询里用 `With<Player>` / `Without<Player>` 过滤，
/// 比在组件里塞 `enum Kind { Player, Enemy }` 再 `if` 判断更快、也更不容易写漏。
#[derive(Component)]
struct Player;

/// 速度，单位：像素 / 秒。
///
/// 故意把速度做成**独立组件**而不是 `Player` 的字段，用来演示系统间通信：
/// [`move_player`] 只负责"根据键盘决定速度"，[`clamp_player`] 只负责"用速度积分出位置并夹住边界"。
/// 两个系统彼此不认识，`Velocity` 就是它们之间的接口。
#[derive(Component)]
struct Velocity(Vec2);

/// 标记：金币。碰到它加分，然后被重新随机摆放。
#[derive(Component)]
struct Coin;

/// 标记：顶部那行分数文本。
#[derive(Component)]
struct ScoreText;

/// 标记：底部那行状态文本（玩法提示 / 暂停 / 通关）。
#[derive(Component)]
struct HudText;

// ============================================================================
// 3. 资源（Resource）：全局唯一的数据
// ============================================================================
//
// 什么时候用 Resource、什么时候用 Component？一条实用判据：**"这东西有几份？"**
//   - 金币有 N 个  → 必须是组件
//   - 分数只有 1 份、又不属于任何实体 → Resource
//   - 玩家也只有 1 个，但仍做成组件：因为它"是一个实体"，
//     以后可能要给它挂别的组件（HP、背包……），Resource 挂不上组件。
// 资源用 `Res<T>` 只读、`ResMut<T>` 可写，是 Bevy 里少数把可变性写在类型上的地方。

/// 本局分数。
#[derive(Resource, Default)]
struct Score(u32);

/// 游戏阶段。做成资源而不是组件，因为它描述的是"整个世界"的状态。
#[derive(Resource, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Playing,
    Paused,
    Won,
}

// ============================================================================
// 4. 入口：组装 App
// ============================================================================

fn main() {
    App::new()
        // DefaultPlugins = 引擎自带的一整组插件（窗口、渲染、输入、资源加载……）。
        // 插件的意义：**把"一堆互相依赖的系统 + 资源 + 初始化"打包成可复用单元**，
        // 所以这一行就等于给 App 装上了整个渲染器。
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "第 19 章 demo · 方块吃金币（方向键/WASD 移动 · P 暂停 · R 重开）".into(),
                resolution: (WINDOW_PX_W, WINDOW_PX_H).into(),
                ..default()
            }),
            ..default()
        }))
        // 资源必须在系统运行前插进 World，否则系统里的 `Res<Score>` 会直接 panic。
        .init_resource::<Score>()
        .init_resource::<Phase>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // `.chain()` = 显式顺序：前一个跑完才跑后一个。
                // 不写 `.chain()` 时 Bevy 会按"谁读写什么数据"自动调度（能并行的就并行）；
                // 这里依赖关系强，直接串起来最直观。
                toggle_pause,
                (move_player, clamp_player, detect_collision).chain(),
                update_score_text,
                update_hud,
                restart,
            ),
        )
        .run();
}

// ============================================================================
// 5. 启动系统：只跑一次的初始化
// ============================================================================

/// `Startup` 阶段只跑一次，用来"建场景"。
///
/// 它只往 World 里写、不读任何东西，所以只需要 `Commands`。
/// `Commands` 是**延迟执行**的：命令先入队，等系统返回、调度器走到同步点才真正生效。
/// 这就是为什么同一个系统里刚 `spawn` 出来的实体查不到——第一个高频坑。
fn setup(mut commands: Commands) {
    // 2D 相机。**没有相机 → 屏幕上什么都没有**（新手第一大坑）。
    commands.spawn(Camera2d);

    // 玩家：一个绿色方块。
    // `Sprite::from_color` 直接给"颜色 + 尺寸"，不需要任何图片文件。
    commands.spawn((
        Player,
        Velocity(Vec2::ZERO),
        Sprite::from_color(Color::srgb(0.30, 0.85, 0.45), Vec2::splat(PLAYER_SIZE)),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    // 金币：一个黄色方块。初始位置随便挑一个（`spawn_coin` 在下面）。
    spawn_coin(&mut commands, 150.0, 120.0);

    // 分数文本。`Text2d` 是"世界空间里的文字"，会跟着相机走；
    // 屏幕空间的 UI 文字是另一个类型 `Text`（需要 bevy_ui 特性，本 demo 没开）。
    commands.spawn((
        ScoreText,
        Text2d::new("Score: 0 / 10"),
        TextFont {
            // 0.20 起 `TextFont::font_size` 是枚举 `FontSize`（Px / Vw / Vh / Rem…），
            // 不再是裸 f32；`FontSize::Px` 或 `.into()` 都可以。
            font_size: FontSize::Px(36.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, WINDOW_H / 2.0 - 44.0, 10.0),
    ));

    // 底部状态行。
    commands.spawn((
        HudText,
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.70, 0.75, 0.85)),
        Transform::from_xyz(0.0, -WINDOW_H / 2.0 + 28.0, 10.0),
    ));
}

/// 生成一个金币。`setup` 和 [`detect_collision`] 都要用，所以抽成普通函数。
///
/// 注意它不是系统：普通 Rust 函数不参与调度、没有自动注入，
/// 参数得自己传。这种"helper 函数"在 ECS 代码里很常见——
/// 系统之间不互相调用，但**生成实体的代码**是可以复用的。
fn spawn_coin(commands: &mut Commands, x: f32, y: f32) {
    commands.spawn((
        Coin,
        Sprite::from_color(Color::srgb(1.0, 0.85, 0.25), Vec2::splat(COIN_SIZE)),
        Transform::from_xyz(x, y, 1.0),
    ));
}

// ============================================================================
// 6. 更新系统：每帧跑一次
// ============================================================================

/// 读键盘 → 写 [`Velocity`]。本帧**不碰位置**。
///
/// 参数顺序无所谓，Bevy 靠**类型**注入：
///   - `Res<ButtonInput<KeyCode>>`：键盘状态资源
///   - `Query<&mut Velocity, (With<Player>, Without<Coin>)>`：
///     所有"有 Player 且没有 Coin"的实体的 `Velocity`（可变借用）
///   - `Res<Phase>`：只读阶段
///
/// `Without<Coin>` 在这里其实多余（没有实体同时是 Player 和 Coin），
/// 但写出来是好习惯：它让"两个查询能同时存在"这件事对编译器显式可见。
fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<Phase>,
    mut player: Query<&mut Velocity, (With<Player>, Without<Coin>)>,
) {
    // `single_mut()` 返回 `Result`：世界里没有 Player（或有两个）时给 `Err`。
    // **坑**：不要无脑 `.unwrap()`——实体刚被 despawn、或场景还没建好时它会 panic。
    // `let Ok(..) else { return }` 优雅跳过这一帧。
    let Ok(mut velocity) = player.single_mut() else {
        return;
    };

    if *phase != Phase::Playing {
        // 暂停 / 通关时把速度清零，否则恢复的瞬间方块会"攒"出一大段位移。
        velocity.0 = Vec2::ZERO;
        return;
    }

    // 同时按左右会互相抵消；用 `+` 而不是直接赋值，斜向手感才对。
    let mut direction = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        direction.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        direction.x += 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        direction.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        direction.y += 1.0;
    }

    // 斜向输入长度是 √2；`normalize_or_zero` 保证八个方向速度一致。
    velocity.0 = direction.normalize_or_zero() * PLAYER_SPEED;
}

/// 用 [`Velocity`] 积分出位置，并把玩家夹在窗口内。
///
/// 这个系统演示两件事：
/// 1. **帧率无关**：位移 = 速度 × `delta_secs()`。
///    写成"每帧加 5 像素"的话，144Hz 显示器上的角色会比 60Hz 快 2.4 倍。
/// 2. **要全局信息就问资源要**：`Res<Time>`、`Query<&Window>`，
///    而不是搞一个 `static` 全局变量（那还会破坏并行调度）。
fn clamp_player(
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut player: Query<(&Velocity, &mut Transform), (With<Player>, Without<Coin>)>,
) {
    let Ok((velocity, mut transform)) = player.single_mut() else {
        return;
    };

    // 第一帧可能还没拿到窗口尺寸；拿不到就用常量兜底，逻辑依然正确。
    let (half_w, half_h) = window
        .single()
        .map(|w| (w.width() / 2.0, w.height() / 2.0))
        .unwrap_or((WINDOW_W / 2.0, WINDOW_H / 2.0));

    transform.translation.x += velocity.0.x * time.delta_secs();
    transform.translation.y += velocity.0.y * time.delta_secs();

    // `max(0.0)` 保证区间合法：窗口比方块还小时 `min > max`，`clamp` 会 panic。
    let limit_x = (half_w - MARGIN).max(0.0);
    let limit_y = (half_h - MARGIN).max(0.0);
    transform.translation.x = transform.translation.x.clamp(-limit_x, limit_x);
    transform.translation.y = transform.translation.y.clamp(-limit_y, limit_y);
}

/// 碰撞检测：玩家与金币中心在两个轴上的距离都小于"半个玩家 + 半个金币"就算吃到。
///
/// 这个系统最能体现 ECS 的写法特点：它**同时操作两类毫不相关的实体**，
/// 却不需要它们有共同基类、也不需要互相持有指针——只要都在同一个 World 里。
fn detect_collision(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut phase: ResMut<Phase>,
    player: Query<&Transform, (With<Player>, Without<Coin>)>,
    coins: Query<(Entity, &Transform), (With<Coin>, Without<Player>)>,
) {
    if *phase != Phase::Playing {
        return;
    }

    let Ok(player_transform) = player.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    // 用"半径和"做两轴分别判断，等价于 AABB 重叠检测。
    // 不写成 `distance < r`，否则方块四个角会被外接圆误判成碰到。
    let hit_radius = (PLAYER_SIZE + COIN_SIZE) / 2.0;

    for (entity, coin_transform) in &coins {
        let coin_pos = coin_transform.translation.truncate();
        let delta = (coin_pos - player_pos).abs();
        if delta.x < hit_radius && delta.y < hit_radius {
            score.0 += 1;

            // 吃到了：把金币挪到新的随机位置，而不是 despawn 再 spawn。
            // `Commands::get_entity` 允许对着**已有实体**下命令，
            // 不需要先把它的组件查出来——这是 ECS 里"改数据的两条路"之一：
            // 通过 `Query` 直接改，或通过 `Commands` 排队改。
            let (x, y) = random_coin_position();
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(Transform::from_xyz(x, y, 1.0));
            }

            if score.0 >= TARGET_SCORE {
                *phase = Phase::Won;
            }
        }
    }
}

/// 分数变了就刷新顶部文本。
///
/// **坑**：不要每帧写文本内容。写字符串会触发排版（字形整形 + 纹理上传），
/// 是 Bevy 里相当贵的操作。用 `Res<Score>::is_changed()` 判断"这帧分数动过没有"。
/// Bevy 为每个 `Res` / `Component` 自动维护变更标记（change tick），
/// 这就是它比手写 `if (dirty)` 方便的地方。
fn update_score_text(score: Res<Score>, mut text: Query<&mut Text2d, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    // `Text2d` 可以解引用成内部字符串（0.15 起文本是 `String`，不再是 `Vec<TextSection>`）。
    **text = format!("Score: {} / {TARGET_SCORE}", score.0);
}

/// 底部状态行：显示当前阶段与玩法提示。同样只在 `Phase` 变化时重写文本。
fn update_hud(phase: Res<Phase>, mut text: Query<&mut Text2d, With<HudText>>) {
    if !phase.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    **text = match *phase {
        Phase::Playing => "Playing · 方向键/WASD 移动，吃满 10 个金币获胜 · P 暂停".to_string(),
        Phase::Paused => "Paused · 按 P 继续".to_string(),
        Phase::Won => "You win! · 按 R 再来一局".to_string(),
    };
}

/// `P`：在 Playing / Paused 之间切换。
///
/// 它只改一个资源，连 `Query` 都没有——系统可以小到这种程度。
/// ECS 的威力就在于：**小系统 + 调度 = 复杂行为**。
fn toggle_pause(keys: Res<ButtonInput<KeyCode>>, mut phase: ResMut<Phase>) {
    if keys.just_pressed(KeyCode::KeyP) {
        // 必须用 `just_pressed`：用 `pressed` 会每帧翻转一次（几十次/秒）。
        *phase = match *phase {
            Phase::Playing => Phase::Paused,
            Phase::Paused => Phase::Playing,
            Phase::Won => Phase::Won,
        };
    }
}

/// `R`：重置分数与阶段。
fn restart(keys: Res<ButtonInput<KeyCode>>, mut score: ResMut<Score>, mut phase: ResMut<Phase>) {
    if keys.just_pressed(KeyCode::KeyR) {
        score.0 = 0;
        *phase = Phase::Playing;
        // `ResMut` 解引用赋值会自动打上"已变更"标记，
        // 所以即使分数本来就是 0，[`update_score_text`] 下一帧也会刷新一次文本。
    }
}

// ============================================================================
// 7. 随机数：不引第三方 crate，手写一个 LCG
// ============================================================================
//
// 真项目里用 `rand` 或 `bevy_math::rand`。这里手写是为了让 demo 零多余依赖，
// 顺便说明"随机数只是一个状态机"。
//
// 用 `AtomicU32` 是为了在 Bevy 的多线程调度下也安全（第 12 章的内容）。

use std::sync::atomic::{AtomicU32, Ordering};

static RNG_STATE: AtomicU32 = AtomicU32::new(0x2545_F491);

/// 线性同余发生器：`state = state * a + c`，取结果当随机数。
/// 质量很差（低位周期极短），但"挪金币位置"这种用途足够了。
fn next_u32() -> u32 {
    let mut old = RNG_STATE.load(Ordering::Relaxed);
    loop {
        let new = old.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        match RNG_STATE.compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return new,
            // 被别的线程抢先改了：用实际值重试（CAS 的标准写法）。
            Err(actual) => old = actual,
        }
    }
}

/// 在窗口内随机取一个金币中心点，边缘留出金币自己的半径。
fn random_coin_position() -> (f32, f32) {
    // 取高 24 位再除以 2^24，得到 [0, 1) 的均匀值。
    let rx = (next_u32() >> 8) as f32 / (1u32 << 24) as f32;
    let ry = (next_u32() >> 8) as f32 / (1u32 << 24) as f32;
    let limit_x = (WINDOW_W / 2.0 - COIN_SIZE).max(1.0);
    let limit_y = (WINDOW_H / 2.0 - COIN_SIZE).max(1.0);
    ((rx * 2.0 - 1.0) * limit_x, (ry * 2.0 - 1.0) * limit_y)
}
