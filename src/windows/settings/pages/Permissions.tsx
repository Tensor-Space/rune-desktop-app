import { AccessibilitySettings } from "../components/AccessibilitySettings";
import { MicrophoneSettings } from "../components/MicrophoneSettings";

export const Permissions = () => {
  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold text-foreground">Permissions</h1>
      <AccessibilitySettings />
      <MicrophoneSettings />
    </div>
  );
};
