import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UserProfileForm } from "../components/UserProfileForm";
import { Settings } from "../types";

interface UserProfileProps {
  onComplete: () => void;
  isStepComplete?: boolean;
}

export const UserProfile = ({ onComplete }: UserProfileProps) => {
  const [, setHasProfile] = useState(false);

  useEffect(() => {
    const checkProfile = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        const profileComplete = !!(
          settings.user_profile?.name || settings.user_profile?.email
        );
        setHasProfile(profileComplete);
        if (profileComplete) {
          onComplete();
        }
      } catch (error) {
        console.error(error);
      }
    };

    checkProfile();
  }, [onComplete]);

  return (
    <div className="container mx-auto">
      <UserProfileForm
        onProfileSet={(profileSet) => {
          setHasProfile(profileSet);
          if (profileSet) {
            onComplete();
          }
        }}
      />
    </div>
  );
};
