import { AudioSettings } from "../components/AudioSettings";

export const Audio = () => {
  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold text-foreground">Audio Settings</h1>
      <AudioSettings />
    </div>
  );
};
