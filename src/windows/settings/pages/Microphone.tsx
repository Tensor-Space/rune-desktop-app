import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MicrophoneSelection } from "../components/MicrophoneSelection";
import { MicrophonePermission } from "../components/MicrophonePermission";
import { Settings } from "../types";

interface AudioProps {
  onComplete: () => void;
  isStepComplete?: boolean;
}

export const Audio = ({ onComplete }: AudioProps) => {
  const [hasMicPermission, setHasMicPermission] = useState(false);
  const [, setHasSelectedDevice] = useState(false);

  useEffect(() => {
    const checkMicrophoneStatus = async () => {
      try {
        try {
          const stream = await navigator.mediaDevices.getUserMedia({
            audio: true,
          });
          stream.getTracks().forEach((track) => track.stop());
          setHasMicPermission(true);
        } catch (err) {
          setHasMicPermission(false);
        }

        const settings = await invoke<Settings>("get_settings");
        const defaultDevice = settings.audio.default_device;
        const hasDevice = !!defaultDevice;
        setHasSelectedDevice(hasDevice);

        if (hasMicPermission && hasDevice) {
          onComplete();
        }
      } catch (error) {
        console.error(error);
      }
    };

    checkMicrophoneStatus();
  }, [onComplete, hasMicPermission]);

  return (
    <div className="container mx-auto">
      <MicrophonePermission
        onPermissionChange={(permitted) => setHasMicPermission(permitted)}
      />

      {hasMicPermission && (
        <div className="mt-6">
          <MicrophoneSelection
            onDeviceSelected={() => {
              setHasSelectedDevice(true);
              if (hasMicPermission) {
                onComplete();
              }
            }}
          />
        </div>
      )}
    </div>
  );
};
