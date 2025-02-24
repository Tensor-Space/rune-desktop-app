import { HotkeySettings } from "../components/HotkeySettings/HotkeySettings";

export const Shortcuts = () => {
  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold text-foreground">Keyboard Shortcuts</h1>
      <HotkeySettings />
    </div>
  );
};
