# 2024-01-14
- update baseview and nih-plug
- switch from custom wry fork to official wry since it now supports attaching to a raw window handle
  - still need to verify how intercepting keyboard events works now
- drop Editor properly when window is closed (no more memory leaks hopefully)

# 2023-07-11
- start using baseview

# 2023-03-10
- macOS: add support for intercepting keys when the plugin UI is focused
