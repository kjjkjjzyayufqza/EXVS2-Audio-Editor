---
name: Full UI responsive refactor
overview: 将 EXVS2-Audio-Editor 的整个 UI（顶部、左侧列表、中间表格、底部播放器、所有弹窗/Toast/按钮区）做一次响应式重构：布局尺寸全部基于可用空间比例计算，支持手机/窄视图；音频播放/进度/音量等业务逻辑与行为保持不变，仅改 UI 结构与视觉。
todos:
  - id: remove-window-min-size
    content: 调整 `src/main.rs` 的 viewport inner/min size：移除 1000x600 的最小限制，使窗口可缩小到窄视图；保留应用正常启动与图标逻辑
    status: completed
  - id: responsive-shell-layout
    content: 重构整体布局骨架：`src/app.rs` SidePanel 宽度改为比例（随窗口宽度变化），必要时在窄屏提供可折叠/可隐藏侧栏的交互；`TopPanel` 保持行为不变但确保窄屏不拥挤
    status: completed
  - id: file-list-responsive
    content: 重构 `src/ui/file_list.rs`：ScrollArea 高度改为比例；窄屏下按钮/搜索区自动换行；不改变 selection/remove/add 行为
    status: completed
  - id: main-area-top-info-responsive
    content: 重构 `src/ui/main_area/main_area_rendering.rs` 与相关“顶部信息区块”：标题/当前文件信息/提示信息在窄屏下换行与截断策略一致且可读
    status: completed
  - id: table-responsive
    content: 重构 `src/ui/main_area/main_area_table.rs` + `src/ui/main_area/table_renderer.rs`：列宽与行高按可用空间比例计算；窄屏时启用更紧凑布局（关键列优先 + 其余信息折叠/弹出），保持所有按钮回调与排序/多选逻辑不变
    status: completed
  - id: player-responsive
    content: 重构 `src/ui/audio_player/audio_player_component.rs` 与 `audio_controls.rs`：去掉固定面板高度、固定扣减宽度；宽屏/窄屏下布局自适应但仍使用同一套 AudioState 方法与交互行为
    status: completed
  - id: modals-and-toasts-responsive
    content: 重构所有 Window/modal 的 min_width/min_height/default_size 等固定值（Confirm/AddAudio/LoopSettings/GRP/DTON/PROP/Toast），改为基于屏幕可用空间比例；内容区域必要时使用 ScrollArea 防止溢出
    status: completed
  - id: english-comments-and-lints
    content: 将触及文件内中文注释统一改为英文；对改动文件跑 lints 并修复引入的问题（不添加任何 TODO 形式的代码）
    status: completed
---

# Full UI Responsive Refactor Plan (UI-only)

## Scope (你确认的范围)

- **顶部**：菜单栏/提示信息/弹窗（TopPanel 的 modal/Window）。
- **左侧**：File select list（搜索框、滚动列表、Add/Remove）。
- **中间**：主编辑区（标题/信息、Search/OutputPath 区块、Audio table、所有操作按钮）。
- **底部**：Audio player（曲名/类型、播放控制、进度条、音量/静音）。
- **弹窗/对话框**：Add Audio、Loop Settings、Confirm、GRP/DTON/PROP 编辑窗口、Toast。

## Hard constraints

- **UI 尺寸规则**：不使用写死的控件/区域 width/height 来决定布局；改为基于 `available_width/available_height` 的比例计算（egui 没有 CSS %，所以用比例）。
- **功能不变**：所有播放/进度条/seek/音量/静音/各种按钮触发行为保持一致，仅调整布局与视觉。
- **注释语言**：改动到的注释统一用英文（同时清理现有零散中文注释）。

## 关键问题（必须先解决，否则无法“窄视图”）

- 主窗口目前被限制为 **最小 1000×600**，会直接阻止窄视图：
```221:231:E:\research\EXVS2-Audio-Editor\src\main.rs
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
```


## Existing fixed-size hotspots (UI-only)

- **侧栏宽度写死**：
```70:85:E:\research\EXVS2-Audio-Editor\src\app.rs
        egui::SidePanel::left("file_list_panel")
            .resizable(true)
            .min_width(200.0)
            .default_width(350.0)
            .show(ctx, |ui| {
                // ...
            });
```

- **FileList 滚动区最大高度写死**：
```127:133:E:\research\EXVS2-Audio-Editor\src\ui\file_list.rs
                ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        // ...
                    });
```

- **底部播放器固定高度 + 进度条固定扣减**：
```39:52:E:\research\EXVS2-Audio-Editor\src\ui\audio_player\audio_player_component.rs
        egui::TopBottomPanel::bottom("audio_player_panel")
            .min_height(120.0)
            // ...
```
```125:152:E:\research\EXVS2-Audio-Editor\src\ui\audio_player\audio_controls.rs
                    ui.allocate_ui_with_layout(
                        Vec2::new(available_width - 150.0, 20.0),
                        // ...
```

- **表格 renderer 使用固定表头/行高/列宽基准**（需要按比例与可用空间重算，避免窄屏溢出）：
```39:53:E:\research\EXVS2-Audio-Editor\src\ui\main_area\table_renderer.rs
        let col_width_checkbox = 18.0;
        let remaining_width = (available_width - col_width_checkbox).max(100.0);
        // ...
        ui.painter().rect_filled(
            Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), 35.0)),
            0.0,
            header_bg_color,
        );
```

- **各种弹窗 min_width/min_height 写死**（确认框、Add Audio、Loop Settings、GRP/DTON/PROP）：
```54:61:E:\research\EXVS2-Audio-Editor\src\ui\main_area\confirm_modal.rs
        Window::new(&self.title)
            .min_width(300.0)
            .min_height(150.0)
```
```204:212:E:\research\EXVS2-Audio-Editor\src\ui\main_area\add_audio_modal.rs
        Window::new("Add New Audio File")
            .min_width(400.0)
            .min_height(300.0)
```
```186:194:E:\research\EXVS2-Audio-Editor\src\ui\main_area\loop_settings_modal.rs
            Window::new(&title)
                .min_width(400.0)
                .min_height(300.0)
```
```58:66:E:\research\EXVS2-Audio-Editor\src\ui\main_area\grp_list_modal.rs
        Window::new("Edit GRP List")
            .min_width(760.0)
            .min_height(520.0)
```
```78:85:E:\research\EXVS2-Audio-Editor\src\ui\main_area\dton_tones_modal.rs
        Window::new("Edit DTON Tones")
            .min_width(980.0)
            .resizable(true)
```


## Design approach (保持功能不变的 UI 重构方法)

- **统一引入“响应式尺寸计算”策略**：每个主要 UI 区块都从 `available_width/height` 得到比例尺寸（例如 side panel 占比、按钮区换行阈值、table row height 基于 `ui.spacing().interact_size.y` 与比例上限/下限）。
- **宽屏/窄屏分支**：以 `available_width` 阈值判定布局（例如按钮区从单行变多行、表格从全列显示变为“关键列 + 更多信息弹出/折叠”）。
- **不改行为绑定**：
  - 进度条仍然在 `drag_stopped()` 时调用 `AudioState::set_position`。
  - 播放/暂停/停止/音量/静音继续走同样的 state 方法。
  - 表格按钮回调（play/export/replace/remove）保持相同触发时机与数据。

## Regression checklist (必须验证)

- 播放时：进度与时间持续刷新；暂停/继续符合预期。
- Seek：拖动进度条后再播放能从新位置开始（保持现有“松手才 seek”）。
- 停止：归零并正确更新按钮状态。
- 音量/静音：滑条与按钮互相影响且立即生效。
- 窄屏：窗口拉到很窄时，顶部/侧栏/表格/播放器都不溢出，按钮仍可点。
- 弹窗：在窄屏下不被强制撑出屏幕（允许滚动或自动适配）。