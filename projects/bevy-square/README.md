# bevy-square —— 第 19 章配套 demo

一个**只用 Bevy 内置图形**的 2D 小游戏：方向键 / WASD 移动绿色方块，碰到黄色金币得分，
吃满 10 个金币获胜；`P` 暂停，`R` 重开。全程**不加载任何图片、字体文件**——
方块是 `Sprite::from_color` 生成的纯色矩形，文字用 Bevy 内置字体。

配套教程：[../../docs/04-frameworks/ch19-game-bevy.md](../../docs/04-frameworks/ch19-game-bevy.md)

## 玩法与操作

| 按键 | 作用 |
|---|---|
| `←` `→` `↑` `↓` 或 `W` `A` `S` `D` | 移动（对角线已归一化，八方向速度一致） |
| `P` | 暂停 / 继续（`Phase` 状态机） |
| `R` | 重开一局（分数清零） |
| `Esc` | 退出（winit 默认行为） |

## 代码里覆盖到的 Bevy 概念

| 概念 | 在 `src/main.rs` 里的位置 |
|---|---|
| `App` + `DefaultPlugins` + `WindowPlugin` | `main()` |
| `Startup` / `Update` 调度、`.chain()` | `main()` 的 `add_systems` |
| 组件 `Player` `Velocity` `Coin` `ScoreText` `HudText` | 第 2 节 |
| 资源 `Score` `Phase` | 第 3 节 |
| `Commands` / 延迟生成实体 | `setup`、`spawn_coin`、`detect_collision` |
| `Query` 过滤（`With` / `Without`）、`single` / `single_mut` | `move_player`、`clamp_player` |
| `Res` / `ResMut`、`is_changed()` 变更检测 | `update_score_text`、`update_hud` |
| `ButtonInput<KeyCode>`、`just_pressed` vs `pressed` | `move_player`、`toggle_pause`、`restart` |
| `Res<Time>` + `delta_secs()` 做帧率无关移动 | `clamp_player` |
| `Transform` + `Sprite` + `Camera2d` | `setup` |
| `Text2d` + `TextFont` + `TextColor`（内置字体） | `setup`、`update_score_text` |
| 多线程安全的自定义随机数（`AtomicU32`） | 第 7 节 |
| 访问 `Window`（`PrimaryWindow`）做边界钳制 | `clamp_player` |

## 怎么跑

```bash
cd projects/bevy-square
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/projects/bevy-square/.cargo-home   # 见下方说明
cargo run
```

**需要图形环境**（X11 或 Wayland）。第一次 `cargo build` 要下载并编译 Bevy 的整条依赖链；
后续增量编译只需几秒。

### 关于 `.cargo/config.toml`

本目录下有一个 `.cargo/config.toml`，做两件事：**（1）把 crates.io 换成 rsproxy 镜像加速下载**、
（2）声明本 crate 用独立的 `CARGO_HOME`（因为本机 `~/.cargo` 只读，而多个 demo 共用
`/home/yxb/code/rs-study/.cargo-home` 时会在 cargo 的 package-cache 文件锁上互相阻塞）。

**在普通机器上不需要它，删掉 `.cargo/` 目录即可**，`cargo run` 会用默认配置从 crates.io 下载。

## 本机实测结果（2026-10-12）

环境：Linux x86_64 / 16 核 / `rustc 1.98.1` / `~/.cargo` 只读 / 有 `DISPLAY=:0` 和 `WAYLAND_DISPLAY`。

```
$ cargo add bevy
      Adding bevy v0.20.0 to dependencies        # ← 实际解析到的版本

$ cargo build          # 本 crate 专用的 CARGO_HOME + rsproxy 镜像
   Compiling bevy-square v0.1.0 (/home/yxb/code/rs-study/projects/bevy-square)
    Finished `dev` profile [optimized + debuginfo] target(s) in 7m 39s
   # 退出码 0，**没有任何 error / warning**

$ cargo fmt --check     # 无输出 = 格式干净

$ timeout 40 cargo run  # 实测确实能开窗，节选日志：
   INFO bevy_render::renderer: AdapterInfo { name: "llvmpipe (LLVM 15.0.7, 256 bits)",
        device_type: Cpu, backend: Vulkan, ... }
   WARN bevy_render::renderer: The selected adapter is using a driver that only
        supports software rendering. This is likely to be very slow.
   INFO bevy_winit::system: Creating new window 第 19 章 demo · 方块吃金币 … (63v0)
   INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported
   ERROR sctk_adwaita::config: XDG Settings Portal did not return response in time
   ICU4X data error: No segmentation model for complex script: Chinese/Japanese
```

结论：

1. **构建与运行都成功**。第一次 `cargo check` 只报出 **1 个 API 错误**——`resolution: (800.0, 600.0)`
   编译不过，因为 `WindowResolution: From<(u32, u32)>` 只接受整数像素（0.14 时代可以传 `Vec2`）。
   改成 `(800u32, 600u32)` 后即通过；这说明本机**不缺 alsa / X11 头文件**，
   需要的只是 `libudev-dev`（手柄）——而我们的精简 feature 正好不带它。
2. 这台机器**没有 GPU，只有 Mesa 的软件光栅器 llvmpipe**（`device_type: Cpu`），
   所以窗口能开、画面能出，但帧率很低。你在有独显的机器上跑会快得多。
3. `ICU4X data error: No segmentation model for complex script: Chinese/Japanese`
   是因为窗口标题里写了中文；**不影响渲染**（文本内容是纯 ASCII 的 `Score: 0 / 10` 等，用内置字体）。
   想让日志干净就把标题换成英文。
4. `Failed to create /home/yxb/.cache/mesa_shader_cache ... Read-only file system`
   是本沙箱 `$HOME` 只读导致的，与代码无关（首次运行要多编译几个 shader）。


## 构建准备（系统依赖）

Bevy 的 `default` 特性包含音频、手柄、UI 和 3D，会拉进需要系统开发库的 crate。
Ubuntu / Debian 上完整装一遍：

```bash
sudo apt install -y build-essential pkg-config \
    libasound2-dev libudev-dev libx11-dev libxcursor-dev libxi-dev \
    libxrandr-dev libxkbcommon-dev libwayland-dev libvulkan1 mesa-vulkan-drivers
```

| 缺什么 | 报错关键词 | 对应包 |
|---|---|---|
| 音频（`bevy_audio` → `alsa-sys`） | `Package alsa was not found` | `libasound2-dev` |
| 手柄（`bevy_gilrs` → `libudev-sys`） | `Package libudev was not found` | `libudev-dev` |
| 窗口 / X11（`winit`） | `Package x11 not found` | `libx11-dev`（再加 `libxcursor-dev` `libxi-dev` `libxrandr-dev`） |
| Wayland 后端 | `Package wayland-client not found` | `libwayland-dev` + `libxkbcommon-dev` |
| 运行时 Vulkan | `No suitable GPU adapters found` | `libvulkan1 mesa-vulkan-drivers` |

### 如果装不了系统库：精简特性

`Cargo.toml` 里这份配置**已经**关掉了 `default`，只开窗口 + 2D 精灵 + 文字所需的特性，
因此不需要 `libasound2-dev`（音频）和 `libudev-dev`（手柄）：

```toml
[dependencies]
bevy = { version = "0.20.0", default-features = false, features = [
    "std", "async_executor", "multi_threaded",
    "bevy_asset", "bevy_log", "bevy_state", "reflect_auto_register",
    "bevy_window", "bevy_winit", "x11", "wayland",
    "bevy_render", "bevy_core_pipeline", "bevy_image", "bevy_color", "png",
    "bevy_sprite", "bevy_sprite_render",
    "bevy_text", "default_font",
    "keyboard", "mouse",
] }
```

关键点：

- **不要**用 `features = ["2d"]` 这种伞形特性——它内部的 `default_platform`
  会带上 `bevy_gilrs`（手柄），从而需要 `libudev-dev`。
- `default_font` 是让文字不依赖外部 `.ttf` 的关键。
- 用 `default-features = false` 时**必须**显式开 `std`，否则会退化到 `no_std` 配置。

## 已知限制

- 没有做「金币新位置离上一次太近」的重试，极小概率两个金币刷在同一个点（练习 19-2 会让你修）。
- 没有音效（需要音频特性 + `libasound2-dev`），也没有存档。
- 逻辑分辨率固定 800×600，不随窗口拉伸自适应。
- **实测状态**：`cargo build` 通过、`cargo run` 成功创建窗口（日志见上），
  但因为本机只有软件光栅器 llvmpipe，没有实际玩过、没有验证帧率；
  「吃到金币 → 加分 → 金币换位置」这条链路建议你在有 GPU 的机器上手动确认一遍。

## 改造练习

见教程文档的「本章练习」：加敌人（19-1）、加排行榜（19-2）、把 `Phase` 换成真正的 `States`（19-3）。
