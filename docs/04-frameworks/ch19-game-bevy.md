# 第 19 章 · 游戏：Bevy

> **一句话定位**：Bevy 是**ECS 优先**的游戏引擎——它不是在引擎里"顺便"用了 ECS，
> 而是**整个 API 都长在 Entity / Component / System 之上**；学会用"系统"而不是"类"思考，就学会了 Bevy。
> **预计时间**：150 分钟（读 70 + 跑/改 demo 80）
> **前置**：第 5 章（结构体与枚举）、第 9 章（泛型与 trait）、第 11 章（智能指针 `Arc`/`Mutex`）、第 12 章（并发）
> **配套 demo**：[`projects/bevy-square`](../../projects/bevy-square/README.md)（独立 crate，不在任何 workspace 里；需要网络拉依赖、需要图形环境）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一个需求「让方块跟着键盘动」，各技术栈的写法几乎没有相似之处：

```c
/* C + SDL2：手写主循环、状态全在全局变量里；速度写成"每帧 0.3"→ 换台电脑手感就变了 */
while (running) {
    while (SDL_PollEvent(&e)) { if (e.type == SDL_QUIT) running = false; }
    const Uint8 *keys = SDL_GetKeyboardState(NULL);          /* 全局键盘数组 */
    if (keys[SDL_SCANCODE_LEFT]) player_x -= 0.3f;
    SDL_RenderClear(ren); /* ...画矩形... */ SDL_RenderPresent(ren);
}
```

```python
# Python + pygame：主循环 + 对象列表；"谁的 update 先跑"靠手写顺序
while running:
    for event in pygame.event.get():
        if event.type == pygame.QUIT: running = False
    if pygame.key.get_pressed()[pygame.K_LEFT]: player.rect.x -= 6
    player.update(); coin.update()               # 对象自己的方法改自己的状态
    if player.rect.colliderect(coin.rect): score += 1
```

```rust
// Rust + Bevy：没有主循环、没有 player.update()；只声明"什么数据"和"什么规则"
fn main() {
    App::new().add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, move_player).run();   // ← 不再有 while 循环
}
fn move_player(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut q: Query<&mut Transform, With<Player>>) {
    let Ok(mut tf) = q.single_mut() else { return };
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft) { dir.x -= 1.0; }
    tf.translation += (dir.normalize_or_zero() * 320.0 * time.delta_secs()).extend(0.0);
}
```

| 维度 | C / C++（SDL、Unreal） | Python（pygame） | Rust（Bevy） |
|---|---|---|---|
| 主循环 | 你自己写（Unreal 用 `Tick`） | 你自己写 | **引擎的**，你只往 `Update` 里塞系统 |
| 组织单位 | 类 / 继承 `GameObject` | 类 + 对象列表 | 组件 + 系统，**没有基类** |
| 遍历所有敌人 | `for (auto *e : enemies) e->update()` | 同左 | `Query<&mut Transform, With<Enemy>>` |
| 数据竞争 | 自己保证（或直接踩坑） | GIL 兜着 | **系统签名**在编译期表达读写意图 |
| 帧率无关 | 自己乘 `dt`，容易忘 | 同左 | `Res<Time>` 注入，仍要自己乘（但很容易拿到） |

Rust 里还有两个同量级的选择：**Macroquad**（立即模式，像 raylib，两小时能写出小游戏）、
**Fyrox**（传统 OOP + 可视化场景编辑器，类 Unity）。想做长线项目、愿意接受数据驱动思维 → Bevy。

> ⚠️ **坑**：Bevy 的 API **几乎每个小版本都在变**。教程里的 `SpriteBundle`、`Camera2dBundle`、`TextBundle`
> 在 0.15 之后都被删了。**别背 API，查你这个版本的文档。**

---

## 1. ECS：本章最重要的一节

ECS = **E**ntity + **C**omponent + **S**ystem。它不是 Bevy 发明的（Unity DOTS、Unreal Mass 都在做），
但 Bevy 是少数**从头到尾都在用 ECS 表达自己**的主流引擎——连相机、窗口、UI 都是组件和系统。

### 1.1 Entity 只是一个 ID

`Entity` 是带 generation 的 64 位整数，**没有数据、没有方法**：

```rust
let e: Entity = commands.spawn(Player).id();   // 记下来的只是一个编号
// e.hp -= 1;   // ❌ 不存在这种写法：实体本身什么都不知道
```

Python 里 `player = Player(); player.hp -= 1`，`player` 身上既有数据又有方法。
ECS 把这两件事拆开：数据在**组件**里，方法在**系统**里，`Entity` 只是把它们串起来的钥匙。

### 1.2 Component：纯数据，挂在实体上

```rust
#[derive(Component)] struct Player;            // 标记组件：没有字段，只用来"打标签"
#[derive(Component)] struct Velocity(Vec2);    // 数据组件
#[derive(Component)]
#[require(Transform, Visibility)]              // 0.15+：挂上它时自动补齐依赖组件
struct Enemy;
```

实体与组件是**多对多**：一个实体能有任意多个组件，一个组件类型能挂在任意多个实体上。
这就是"组合优于继承"最彻底的形态——想加"能被点燃"的能力，就贴一个 `Flammable` 组件，
而不是让 `Player` 和 `Enemy` 都去继承 `FlammableObject`。
**继承是纵向的（一层层加能力），组合是横向的（按需贴标签）。**

### 1.3 System：普通函数，签名即依赖声明

```rust
fn gravity(time: Res<Time>, mut q: Query<&mut Velocity, With<PhysicsBody>>) {
    for mut v in &mut q {
        v.0.y -= 980.0 * time.delta_secs();    // 单位是像素/秒 → 帧率无关
    }
}
```

| 参数类型 | 拿到的 | 能改吗 |
|---|---|---|
| `Res<T>` / `ResMut<T>` | 全局唯一的资源 | ❌ / ✅ |
| `Query<&T, F>` / `Query<&mut T, F>` | 一组实体上的组件 | ❌ / ✅ |
| `Query<Entity, F>` | 实体 ID 本身 | — |
| `Commands` | 延迟执行的结构变更队列 | 增删实体/组件 |
| `EventReader<E>` / `EventWriter<E>` | 读 / 写事件 | — |
| `Local<T>` | 只属于这个系统的跨帧状态 | ✅ |

**为什么系统之间不需要互相调用？** 因为信息通过 World 传递：
`move_player` 写 `Velocity`，`clamp_player` 读 `Velocity` 写 `Transform`，`detect_collision` 读 `Transform` 写 `Score`。
四个函数互不相识，甚至可以分散在不同 crate 里。这是 ECS 最重要的收益：
**新增行为 = 新增一个系统，而不是修改已有代码**（开闭原则落地）。

### 1.4 Resource：全局唯一的数据

判据是"这东西有几份"：分数只有一份、又不属于任何实体 → `Res<Score>` / `ResMut<Score>`；
金币有 N 个 → 必须是组件；玩家虽然也只有一个，但它**是一个实体**（以后要挂 HP、背包），所以做组件。

```rust
#[derive(Resource, Default)] struct Score(u32);
app.init_resource::<Score>();     // 忘了注册 → 系统里的 Res<Score> 直接 panic
```

### 1.5 Query：过滤与组合

```rust
// 所有"有 Player、没有 Coin"的实体，可写 Transform
Query<&mut Transform, (With<Player>, Without<Coin>)>
// 实体 ID + 可写 Velocity，只挑敌人
Query<(Entity, &mut Velocity), With<Enemy>>
// 同时查两类实体：玩家坐标 + 所有金币坐标（互不冲突）
player: Query<&Transform, (With<Player>, Without<Coin>)>,
coins:  Query<(Entity, &Transform), (With<Coin>, Without<Player>)>,
```

`With` / `Without` 是**过滤器**：不取出数据，只做筛选，运行时几乎免费；
它还能让调度器确认"这两个查询不冲突"，从而允许并行。

> ⚠️ **坑**：`Query` 找不到实体**不报错**，`for x in &q` 或 `q.iter()` 直接空转。
> "我的系统没生效"十有八九是过滤器写错，或者实体根本没生成。

### 1.6 Schedule：谁先跑、能不能并行

系统由**调度器**（`Schedule`）驱动。App 内置阶段：

| 阶段 | 什么时候 | 典型用途 |
|---|---|---|
| `PreStartup` / `Startup` / `PostStartup` | 启动时各一次 | 建场景、加载资源 |
| `First` → `PreUpdate` | 每帧开头 | 输入预处理、输入映射 |
| `Update` | 每帧主战场 | 99% 的游戏逻辑 |
| `PostUpdate` | 变换传播之后 | 同步物理结果 |
| `Last` | 每帧最后 | 清理、统计 |
| `FixedUpdate` | 固定时间步（默认 64Hz） | 物理积分：要可复现就用它 |

同一阶段内，**系统按"读写冲突"自动并行**：两个都只读 `Score`，可以在两个核上同时跑；
一个要 `ResMut<Score>`，调度器就让它们错开。`.chain()` 是手动兜底：

```rust
.add_systems(Update, (move_player, clamp_player, detect_collision).chain())
```

> ⚠️ **坑**：**两个系统**同时要 `ResMut<Score>` 是合法的（调度器串起来就行），
> 但**同一个系统内部**出现两个 `ResMut<Score>` 是**运行时报错**（不是编译错误！）。
> 同理，同一系统里两个 `Query` 若可能命中同一实体的 `&mut Transform`，也会 panic。
> 解法是拆系统 + `.chain()`——这正是 demo 把三个移动相关系统拆开的原因。

### 1.7 数据驱动与缓存友好

OOP 场景图里每个 `Enemy` 是一次堆分配，`hp`、`transform`、`sprite` 散落在对象内部不同偏移；
遍历 10000 个敌人做"移动"，CPU 会不停跳内存、刷缓存行。
ECS 反过来：**同类型组件按列存储**，`Query<&mut Transform, With<Enemy>>` 拿到的是一段连续数组，
移动 10000 个敌人就是顺序扫内存。这是 ECS"数据驱动"的字面含义：
**不是"对象调用方法"，而是"数据被批量施加规则"。**
代价是你写不出 `enemy.attack(player)`，得写一个系统、声明"我要敌人的攻击力和玩家的血量"；
回报是行为可任意组合、可被调度器并行、可热重载，而且**没有任何 `GameObject` 基类**。

---

## 2. Bevy 工程结构

### 2.1 Plugin：把代码组织成可插拔单元

```rust
struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
           .add_systems(Startup, spawn_hud)
           .add_systems(Update, (detect_collision, update_score_text).chain());
    }
}
app.add_plugins((DefaultPlugins, ScorePlugin));
```

`DefaultPlugins` 自己就是一组插件的集合（由 `plugin_group!` 宏生成），
所以"给 App 装上整个渲染器"只要一行。**这是 Bevy 唯一推荐的代码组织方式**：
按功能切文件，每个文件导出一个 `Plugin`；不要按 `Player.rs` / `Enemy.rs` 这种类型切。

### 2.2 States：状态机是一等公民

demo 用 `enum Phase` 资源手写了一个简化状态机；真项目应该用 `States`：

```rust
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState { #[default] Menu, Playing, Paused }

app.init_state::<GameState>()
   .add_systems(OnEnter(GameState::Playing), spawn_level)
   .add_systems(OnExit(GameState::Playing), cleanup_level)
   .add_systems(Update, move_player.run_if(in_state(GameState::Playing)));
```

好处：`OnEnter`/`OnExit` 挂钩子、`run_if(in_state(..))` 自动开关系统、
状态变更是 `StateTransition` 事件、配合 `DespawnOnExit`（旧名 `StateScoped`）可**自动销毁**该状态下的实体。

### 2.3 Event：一对多通信

```rust
#[derive(Event)] struct AteCoin { position: Vec2 }

fn detect_collision(mut events: EventWriter<AteCoin>) { events.write(AteCoin { position }); }
fn play_sound(mut events: EventReader<AteCoin>) {       // 可以有多个读者！
    for ev in events.read() { info!("吃到金币 at {:?}", ev.position); }
}
app.add_event::<AteCoin>();
```

事件是**一对多、发送方不关心谁在听**：加音效、加屏幕震动、加日志只需新增一个读者，
`detect_collision` 一行不改。注意事件默认**只保留两帧**，没人读就丢弃。

### 2.4 资源加载、UI 与时间

```rust
let texture = asset_server.load("sprites/player.png");   // 立即返回 Handle，异步加载
```

`load` **不等文件读完**，返回引用计数的 `Handle<T>`。这也是 demo 故意**不用任何外部文件**的原因：
少一个"路径写错/文件不存在"的失败点。

| | 世界空间（跟随相机） | 屏幕空间（HUD） |
|---|---|---|
| 文字类型 | `Text2d` | `Text`（UI 节点） |
| 需要特性 | `bevy_sprite` + `bevy_text` | `bevy_ui` + `bevy_ui_render` |
| 定位 | `Transform` | `Node { position_type, left, top, .. }` |
| demo 里 | 分数、状态行都用 `Text2d` | 未使用（省掉 `bevy_ui` 依赖） |

**文字必须给 `TextFont`**：0.20 起 `font_size` 是枚举 `FontSize`（`Px`/`Vw`/`Vh`/`Rem`）而不是裸 `f32`；
不指定字体时用 `default_font` 特性内置的 `FiraMono-subset.ttf`——**不需要任何 ttf 文件**。
时间 API 则有 `time.delta_secs()`（帧率无关移动）、`time.elapsed_secs()`（动画计时）、
`FixedUpdate` + `Res<Time<Fixed>>`（固定 1/64 秒步长，物理积分/回放）。
⚠️ 写游戏最容易犯的错就是 `x += 5.0`：144Hz 屏幕上角色会比 60Hz 快 2.4 倍。

---

## 与 demo 的对照

打开 [`projects/bevy-square/src/main.rs`](../../projects/bevy-square/src/main.rs)，逐段对上：

| demo 里的东西 | 对应本章 | 关键代码 |
|---|---|---|
| `main()` | §2.1 | `App::new().add_plugins(DefaultPlugins.set(WindowPlugin{..}))` |
| `Player` `Velocity` `Coin` `ScoreText` `HudText` | §1.2 | 全在 `#[derive(Component)]` 下，无方法 |
| `Score` `Phase` | §1.4 | 用之前必须 `init_resource` |
| `setup` | §1.6 | `Startup` 阶段；`commands.spawn(Camera2d)` |
| `move_player` | §1.3 | `Res<ButtonInput<KeyCode>>` → `Query<&mut Velocity, With<Player>>` |
| `clamp_player` | §1.3 / §2.4 | `delta_secs()` 积分位移；`Query<&Window, With<PrimaryWindow>>` 取边界 |
| `detect_collision` | §1.5 | 两轴距离判定；`commands.get_entity(e)?.insert(..)` 换金币位置 |
| `update_score_text` / `update_hud` | §2.4 | `Query<&mut Text2d, With<ScoreText>>` + `is_changed()` |
| `toggle_pause` / `restart` | §2.2（手写版） | `Phase` 资源；练习 19-3 会换成真正的 `States` |
| `.chain()` / 手写 LCG 随机数 | §1.6 / §1.7 | `(...).chain()`；`AtomicU32` + `compare_exchange_weak` |

三个值得反复看的写法：

1. **`let Ok(..) = q.single_mut() else { return };`** —— 别 `unwrap()`：
   场景没建好、实体刚被删，都会让它返回 `Err`。
2. **`Velocity` 是独立组件** —— 专门用来演示"系统之间靠数据通信"。
   把速度塞进 `Player` 也能跑，但那样两个系统就得共享同一个组件类型，拆分能力反而变差。
3. **`is_changed()` 才重写文本** —— 每帧写字符串会触发文本重排版，是 Bevy 里比较贵的一步。

---

## 构建准备

### 本机的实测结果

> 以下是本机（Linux x86_64、16 核、`~/.cargo` 只读、有 `DISPLAY=:0`）的实测，输出原样保留。

```bash
cd /home/yxb/code/rs-study/projects/bevy-square
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/projects/bevy-square/.cargo-home
cargo add bevy     #  Adding bevy v0.20.0 to dependencies    ← 实际解析到的版本
cargo build        #  Finished dev profile ... in 7m 39s     ← 退出码 0，无 error / warning
cargo fmt --check  #  无输出 = 格式干净
```

**整个过程只报过 1 个编译错误，很值得记下来**：

```text
error[E0277]: the trait bound `WindowResolution: From<(f32, f32)>` is not satisfied
```

`WindowPlugin` 的 `resolution` 在 0.20 只接受 `From<(u32, u32)>`（物理像素整数），
而 0.13/0.14 时代的写法是 `resolution: (800., 600.).into()`（那时是 `Vec2`）。
改成 `(800u32, 600u32)` 立刻通过。**这就是"别背 API"的具体含义。**

系统依赖方面：本机 `alsa` 1.2.6.1 与 `x11` 1.7.5 的头文件**都有**，所以窗口能编过；
只有 `libudev-dev` **缺失**（`/usr/include/libudev.h` 不存在），而它只被手柄（`bevy_gilrs`）需要——
demo 从一开始就 `default-features = false`，因此不需要 `libasound2-dev`，也不需要 `libudev-dev`。

`cargo run` 的实测日志（节选）：

```text
INFO bevy_render::renderer: AdapterInfo { name: "llvmpipe (LLVM 15.0.7, 256 bits)",
     device_type: Cpu, backend: Vulkan }
WARN bevy_render::renderer: The selected adapter is using a driver that only supports
     software rendering. This is likely to be very slow.
INFO bevy_winit::system: Creating new window 第 19 章 demo · 方块吃金币 … (63v0)
ICU4X data error: No segmentation model for complex script: Chinese/Japanese
```

- **窗口确实开出来了**：渲染管线、winit、输入插件都装好了；
- 本机**没有 GPU，只有 Mesa 软件光栅器 llvmpipe**（`device_type: Cpu`），帧率会很低；
- `ICU4X` 那行是窗口标题里的中文触发的（0.20 文字后端换成了 Parley/ICU4X），
  **不影响渲染**——屏幕上的文本都是 ASCII 的 `Score: 0 / 10`。想让日志干净就把标题改成英文。

### 系统依赖与 `default-features = false`

Ubuntu / Debian 上装齐 Bevy 全部功能需要：

```bash
sudo apt install -y build-essential pkg-config libasound2-dev libudev-dev \
    libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxkbcommon-dev \
    libwayland-dev libvulkan1 mesa-vulkan-drivers
```

| 缺什么 | 报错关键词 | 对应包 |
|---|---|---|
| 音频（`alsa-sys`） | `Package alsa was not found` | `libasound2-dev` |
| 手柄（`libudev-sys`） | `Package libudev was not found` | `libudev-dev` |
| X11 窗口 | `Package x11 not found` | `libx11-dev`（+ `libxcursor-dev` `libxi-dev` `libxrandr-dev`） |
| Wayland 后端 | `Package wayland-client not found` | `libwayland-dev` `libxkbcommon-dev` |
| 运行时无 GPU | `No suitable GPU adapters found` | `libvulkan1` `mesa-vulkan-drivers` |

如果装不了系统库，就只开需要的特性（**这也是 demo 的配置**）：

```toml
[dependencies]
bevy = { version = "0.20.0", default-features = false, features = [
    "std", "async_executor", "multi_threaded",
    "bevy_asset", "bevy_log", "bevy_state", "reflect_auto_register",
    "bevy_window", "bevy_winit", "x11", "wayland",
    "bevy_render", "bevy_core_pipeline", "bevy_image", "bevy_color", "png",
    "bevy_sprite", "bevy_sprite_render", "bevy_text", "default_font",
    "keyboard", "mouse",
] }
```

三条硬性注意：**（1）必须显式开 `std`**，否则 Bevy 退化成 `no_std` 配置、报一堆找不到类型的错；
**（2）不要用 `features = ["2d"]`**，这个伞形特性里的 `default_platform` 会带上 `bevy_gilrs` → 需要 `libudev-dev`；
**（3）`default_font`** 是"文字不依赖外部 ttf"的关键（`png` 只是渲染管线的格式需求）。

---

## 本章练习

三题都是**改造 demo**，完整验收标准写在 [`projects/bevy-square/README.md`](../../projects/bevy-square/README.md)。

| 题号 | 任务 | 考点 |
|---|---|---|
| 19-1 | 加敌人：`Enemy` + `EnemyVelocity` 组件，自己巡逻；碰到玩家扣分，分数到 0 判负 | 组件拆分、`With`/`Without` 过滤、两个系统操作同类组件 |
| 19-2 | 加排行榜：`Resource` 存最近 5 局分数，`R` 重开时把本局推进去并显示多行 | `Resource` 里的 `Vec`、`is_changed()`、文本重排版成本 |
| 19-3 | 把 `Phase` 资源换成真正的 `States`：`OnEnter`/`OnExit` + `run_if(in_state(..))` + `DespawnOnExit` | 状态机、`run_if` 组合子、状态作用域实体清理 |

**19-1 提示**：敌人别复用 `Velocity`——那样 `Query<&mut Velocity>` 会同时命中玩家和敌人，
你必须加 `Without` 才不会 panic；这正是体验"借用冲突"的好机会。

**19-2 提示**：多行排行榜直接往 `Text2d` 的字符串里写 `\n` 即可。
变更标记只在 `ResMut` 被**解引用赋值**（含 `DerefMut` 的 `push`）时打上。

**19-3 提示**：`init_state::<Phase>()` 之后写
`add_systems(Update, move_player.run_if(in_state(Phase::Playing)))`；
`OnEnter(Phase::Won)` 里生成结算文本并挂 `DespawnOnExit(Phase::Won)`，重开时就不用自己 `despawn`。

验收标准（三题通用）：

- [ ] `cargo build` 通过，没有新增编译警告（`cargo clippy -- -D warnings` 更好）
- [ ] `cargo run` 后新功能**玩得到**，不是"只是能编译"
- [ ] 新系统之间**没有互相调用**——如果 `a()` 里调了 `b()`，说明拆分方式错了
- [ ] 每个新 `Query` 都写了必要的 `With`/`Without`，且没有两个会冲突的 `&mut`

---

## 常见坑

1. **API 变得极快，别背 API**。0.15 删掉 `SpriteBundle`/`Camera2dBundle`/`TextBundle`，改成裸组件 + `#[require(..)]`；
   0.16 改了文本组件；0.20 换成 Parley 后端，`TextFont::font_size` 从 `f32` 变成 `FontSize` 枚举，
   `Text2d` 从 `bevy_text` 挪进 `bevy_sprite`，`WindowResolution` 也不收 `(f32, f32)` 了。
   **报错第一件事是查你这个版本的签名**（`cargo doc --open`）。
2. **同一类型不能有两个可变借用**。同一系统里两个 `ResMut<Score>`、或两个可能命中同一实体的 `Query<&mut Transform>`，
   都会**运行时 panic**——不是编译错误，所以 `cargo check` 过了不代表能跑。判据：同系统的两个查询必须靠 `With`/`Without` 互斥。
3. **`Query` 查不到是静默的**。调试时先打印 `q.iter().count()`，再查：实体生成了吗？过滤器写对了吗？系统加进哪个阶段了？
4. **`default` 特性会拉进需要系统库的 crate**（音频→alsa、手柄→libudev）。无桌面环境第一件事就是 `default-features = false`，并**显式开 `std`**。
5. **忘了 `Camera2d` = 黑屏**。渲染不出东西先查相机，再查 `Transform.z` 是否在相机可见范围内（2D 里 z 越大越靠前，demo 用 1.0~10.0）。
6. **编译慢有办法缓解**：`[profile.dev.package."*"] opt-level = 3` 只让依赖优化，自己的 crate 保持 `opt-level = 1`，改代码不触发依赖重编；
   再快就 `cargo check` 代替 `build`、用 `sccache`，或开发期开 `dynamic_linking`（**别带进 release**）。**开的 feature 越少编译越快**。
7. **热重载（`file_watcher`）只对资产生效**：它监视 `assets/` 里的图片/场景，Rust 代码改了仍要重新编译。
8. **`Commands` 是延迟的**：`commands.spawn(..)` 之后同一系统里查不到这个实体，要在生成时补数据就用链式 `entity.insert(..)`。
9. **跨帧状态用 `Local<T>`，别用 `static mut`**（2024 edition 里后者直接编译错误，还会破坏并行调度）；demo 的随机数状态因此用了 `AtomicU32`。
10. **`Text2d` 的坐标是文本中心**：0.20 里 `Text2d` 自带 `Anchor`、默认中心对齐，所以 `from_xyz(0.0, 300.0, ..)` 是"文字中心在 y=300"。

---

## 自测清单

- [ ] 我能用一句话解释 Entity / Component / System / Resource 各自的职责
- [ ] 我能说清"为什么系统之间不需要互相调用"，并给出至少两条好处
- [ ] 我知道 `Query<&T>` 与 `Query<&mut T>` 的区别，以及 `With`/`Without` 是过滤器而不是取值
- [ ] 我能说出"什么时候用 Resource、什么时候用 Component"的判据
- [ ] 我知道 `Update` 与 `FixedUpdate` 的区别，以及物理模拟该放哪个
- [ ] 我能解释 `.chain()` 与默认自动调度的关系，以及为什么并行是默认行为
- [ ] 我知道同一系统里两个 `ResMut<T>` 会在**运行时** panic（而不是编译期报错）
- [ ] 我能说出 `Commands` 为什么延迟，以及"生成后立刻查询查不到"怎么绕
- [ ] 我知道 `is_changed()` 能省掉不必要的文本重排版；也解释得出 `default-features = false` 后为什么**必须**显式开 `std`
- [ ] 我知道本机 `cargo build` 的实测结果（7m39s 成功、只有 1 个 `WindowResolution` API 错误），以及换台机器该装哪些系统包
- [ ] 我跑通了 demo（本机实测能开窗），并完成了至少一道练习

---

## 附：本章内容的实测状态

| 内容 | 状态 |
|---|---|
| bevy 版本解析为 0.20.0；`cargo build` 成功（7m39s，退出码 0，无 error/warning）；`cargo fmt --check` 干净 | ✅ 实测 |
| `cargo run` 创建窗口成功（llvmpipe 软件渲染，`device_type: Cpu`） | ✅ 实测，但**未人工试玩**（无 GPU，无法判断手感/帧率） |
| `Sprite::from_color` / `Text2d` / `TextFont` / `TextColor` / `Camera2d` / `ButtonInput` / `Query::single(_mut)` / `Commands::get_entity` / `Time::delta_secs` / 文本 `Deref` / `WindowResolution: From<(u32,u32)>` | ✅ 逐个对照 0.20.0 的 crate 源码（`WindowResolution` 是编译器抓出来的） |
| 精简 feature 列表（关掉 `default` 后不需要 alsa / libudev） | ✅ 实测 |
| `States` / `OnEnter` / `OnExit` / `DespawnOnExit` / `AssetServer` / `Event` / `Plugin` 的代码片段 | ⚠️ **纸面讲解**，未在 demo 中启用（demo 用手写 `Phase` 资源演示同一思路） |
| 练习 19-1 / 19-2 / 19-3 的改造 | ⚠️ 纸面，未实现 |
