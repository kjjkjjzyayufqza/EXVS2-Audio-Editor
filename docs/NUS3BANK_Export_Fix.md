# song_wgnmd1.nus3bank 读取与导出修复

## 原因

该文件的 TONE 名称、PACK 偏移和大小字段采用无前缀布局，但后续参数表无法按现有 EXVS2 布局完整解析。旧代码接受了偏移 8 字节的错误候选，产生含有 78 个 NUL 字节的名称及 6 字节的错误大小。单条导出将此名称用于输出路径，导致 Windows 拒绝启动解码命令。

播放使用数字音轨编号和自动生成的临时文件名，因此仍可调用 vgmstream 正常解码。

## 修改

- TONE 名称必须正确终止，且不能含 NUL 或 ASCII 控制字符。
- 完整参数解析失败时，仅接受唯一、通过校验的名称与 PACK 位置头部；读取时继续检查 PACK 范围。未知参数区保留在 `raw_meta`，保存时沿用现有原样写回机制。
- 单条和批量导出的文件名均处理 Windows 非法字符与设备保留名。
- NUS3BANK 的“全部导出”使用 vgmstream 解码为 PCM WAV，不再将压缩数据直接写成 `.wav`；失败会明确报告。
- 文件列表按实际音频头显示 BNSF、WAV 或 Unknown。

此修复针对读取和音频导出；没有实现该变体未知参数区的完整语义编辑。

## 验证与构建

使用 Rust 1.97 或更新版本及 Windows MSVC 构建工具。在项目目录运行：

```powershell
cargo +stable test --locked --lib nus3bank
cargo +stable build --locked
```

新增的固定测试样本只包含 308 字节的 TONE 元数据，不包含歌曲音频。测试覆盖读取、保存后元数据保留、PACK 越界、原有 8 字节前缀布局、非法文件名、PCM8 到 PCM16 解码以及失败报告。

若项目上一级存在原始 `song_wgnmd1.nus3bank`，测试还会验证该文件的单条及批量导出，检查 WAV 为 48 kHz、双声道、16 位、5,158,694 个采样帧。

编译生成的程序位于 `target/debug/exvs2_audio_editor.exe`。运行目录需要包含随项目附带的 `tools` 文件夹。
