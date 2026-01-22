---
name: Responsive audio player UI refactor
overview: 重构 EXVS2-Audio-Editor 的播放器与相关整体布局为响应式（适配手机/窄视图），严格保持现有播放/进度/音量等功能逻辑不变，仅调整 UI 结构与视觉呈现；布局宽高以“基于可用空间的比例”计算，不再写死固定 width/height。
todos:
  - id: map-player-ui
    content: 梳理播放器 UI 结构与所有交互点（progress/seek、play/pause、stop、volume/mute）并确定需要保持的行为边界
    status: pending
  - id: refactor-audio-controls-responsive
    content: 重构 `src/ui/audio_player/audio_controls.rs`：拆分为 header/seek/controls 三段，并按可用宽度做宽屏/窄屏布局分支；移除固定宽高分配
    status: pending
  - id: refactor-audio-player-panel
    content: 重构 `src/ui/audio_player/audio_player_component.rs`：底部面板高度由内容或比例决定，移除固定 min_height；整理并将中文注释改为英文注释
    status: pending
  - id: responsive-shell-layout
    content: 调整整体布局：`src/app.rs` 侧栏宽度改为比例；`src/ui/file_list.rs` ScrollArea 高度改为比例
    status: pending
  - id: responsive-table-and-modals
    content: 将表格与弹窗中的固定宽高改为比例/基于 style 的尺寸：`table_renderer.rs`、`main_area_rendering.rs`、`*_modal.rs` 等
    status: pending
  - id: lint-and-regression
    content: 对改动文件做 lints 检查并修正；按回归清单验证播放/进度/音量在宽屏与窄屏下均正常
    status: pending
---

# Responsive Audio Player UI Refactor Plan

## 目标与约束

- **目标**：重构“底部音频播放器 + 与其耦合的整体布局”，在窄屏/手机宽度下依然可用、信息清晰、操作顺畅。
- **不变项（功能保证）**：播放/暂停、停止、进度条拖拽寻址、播放进度实时更新、音量/静音切换、当前音频信息显示与加载逻辑完全保持。
- **布局规则**：不再使用写死的区域/控件 **width/height**（例如 `120.0`、`400.0`、`35.0` 等）来决定布局大小；改为基于 `ui.available_width()/available_height()` 或 `ctx.available_rect()` 的比例计算。

## 现状切入点（关键硬编码尺寸）

- 底部播放器面板固定最小高度：
```39:52:E:\research\EXVS2-Audio-Editor\src\ui\audio_player\audio_player_component.rs
    /// Show the audio player at the bottom of the screen
    pub fn show(&mut self, ctx: &Context) {
        // Update playback position
        self.update_playback_position();
        
        // Display audio player in a bottom panel
        egui::TopBottomPanel::bottom("audio_player_panel")
            .min_height(120.0)  // Increased height for better UX
            .frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill))
            .resizable(false)
            .show(ctx, |ui| {
                self.render(ui);
            });
    }
```

- 进度条区域用固定减法/固定高度分配：
```125:152:E:\research\EXVS2-Audio-Editor\src\ui\audio_player\audio_controls.rs
                    // Progress slider
                    let mut progress = state_copy.progress();

                    // Calculate available width for the slider
                    let available_width = ui.available_width();

                    // Create a custom sized area for the slider, reserving space for the duration and controls
                    ui.allocate_ui_with_layout(
                        Vec2::new(available_width - 150.0, 20.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            // Add the slider in the allocated space
                            ui.style_mut().spacing.slider_width = ui.available_width() - 70.0;
                            let slider_response = ui.add(
                                Slider::new(&mut progress, 0.0..=1.0)
                                    .show_value(false)
                                    .text(""),
                            );

                            // Only update position if slider has been released to avoid
                            // constant reloading while dragging
                            if slider_response.drag_stopped() && has_audio {
                                let mut state: std::sync::MutexGuard<'_, AudioState> = self.audio_state.lock().unwrap();
                                let new_position = progress * state.total_duration;
                                state.set_position(new_position);
                            }
                        },
                    );
```

- 侧栏与文件列表滚动区域存在固定尺寸：
```70:85:E:\research\EXVS2-Audio-Editor\src\app.rs
        egui::SidePanel::left("file_list_panel")
            .resizable(true)
            .min_width(200.0)
            .default_width(350.0)
            .show(ctx, |ui| {
                // Display file list component
                if self.file_list.show(ui) {
                    // If a file is selected, handle it here
                    if let Some(selected) = &self.file_list.selected_file {
                        println!("Processing file: {}", selected);
                        // Update main area with selected file
                        self.main_area.update_selected_file(Some(selected.clone()));
                    }
                }
            });
```


## 设计方案（UI/UX，不改逻辑）

### 播放器（底部 Dock）

- **布局分层**：将播放器 UI 拆成 3 个区域，并根据可用宽度自适应排列。
  - **Header**：曲名（省略号/换行策略）+ 类型 badge。
  - **Seek**：`当前位置` + 进度 slider（占满剩余宽度）+ `总时长`。
  - **Controls**：播放/暂停、停止、音量 slider、静音。
- **响应式规则（无固定宽高）**：
  - 宽屏：Header 与 Controls 同行（或两行但紧凑）；Seek 单独一行全宽。
  - 窄屏：Header 单独一行；Seek 单独一行全宽；Controls 变成两行（Transport 一行、Volume 一行）或使用 `horizontal_wrapped`。
- **关键实现手段**：
  - 用 `ui.available_width()` / `ui.available_height()` 计算各区域占比（例如 seek 区域全宽、controls 区域在窄屏时自动换行）。
  - 进度条用 `ui.add_sized([seek_width, slider_height], Slider::new(...))`，其中 `seek_width` 来自比例计算/剩余宽度，避免 `available_width - 150.0`。
  - 取消 `TopBottomPanel::min_height(120.0)`，改为由内容自然决定高度，或以 `ctx.available_rect()` 的比例设置默认高度（不写死）。

### 整体布局（与播放器耦合的可用空间）

- **左侧文件列表面板**：将 `min_width/default_width` 改为基于 `ctx.available_rect().width()` 的比例，窄屏时自动收窄。
- **FileList 内部 ScrollArea**：`max_height(400.0)` 改为 `ui.available_height() * ratio`。
- **表格/各类弹窗**：对 `min_width/min_height/default_size/row_height/header_height` 等固定值改为比例或基于 `ui.spacing().interact_size`（属于 UI 基准值，不直接写死宽高）。
  - 受影响文件（从搜索结果看）：
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\table_renderer.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\table_renderer.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\main_area_rendering.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\main_area_rendering.rs)（toast `default_size` 等）
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\add_audio_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\add_audio_modal.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\loop_settings_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\loop_settings_modal.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\confirm_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\confirm_modal.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\grp_list_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\grp_list_modal.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\dton_tones_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\dton_tones_modal.rs)
    - [`E:\research\EXVS2-Audio-Editor\src\ui\main_area\prop_edit_modal.rs`](E:\research\EXVS2-Audio-Editor\src\ui\main_area\prop_edit_modal.rs)

## 不改功能的保障措施

- **保留所有状态与回调路径**：`AudioState` 的 `toggle_play/stop/set_position/set_volume/toggle_mute/update_from_backend` 不做语义修改；UI 只是在不同布局下调用同样的方法。
- **进度条交互保证**：仍然使用 `drag_stopped()` 时写回 position，避免拖拽期间频繁 seek（保持现有行为）。
- **回归检查清单（手动）**：
  - 播放时：时间与进度条持续更新。
  - 暂停时：拖拽进度条后再次播放能从新位置开始。
  - 停止：位置归零、按钮状态正确。
  - 音量：拖动实时生效；静音/恢复保持原逻辑。
  - 窄屏（把窗口拉窄）：控件不溢出、可操作、信息不遮挡。