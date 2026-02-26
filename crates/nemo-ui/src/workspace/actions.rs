use gpui::actions;

actions!(
    nemo,
    [
        ReloadConfig,
        QuitApp,
        CloseProject,
        OpenProject,
        ToggleTheme,
        ShowKeyboardShortcuts,
        OpenSettings,
        CloseSettings,
    ]
);
