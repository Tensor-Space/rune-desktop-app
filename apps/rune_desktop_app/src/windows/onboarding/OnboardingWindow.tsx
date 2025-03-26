import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ProfileSetup } from "./steps/ProfileStep";
import { CompletionStep } from "./steps/CompletionStep";
import { PermissionsSetup } from "./steps/PermissionsStep";
import { Card } from "@/components/ui/card";

enum OnboardingStep {
  PROFILE = 0,
  PERMISSIONS = 1,
  COMPLETION = 2,
}

interface UserProfile {
  name: string;
  email: string;
  about: string;
}

export const OnboardingWindow = () => {
  const [currentStep, setCurrentStep] = useState<OnboardingStep>(
    OnboardingStep.PROFILE,
  );
  const [userProfile, setUserProfile] = useState<UserProfile>({
    name: "",
    email: "",
    about: "",
  });
  const [, setIsPermissionGranted] = useState(false);

  const saveUserProfile = async (profile: UserProfile) => {
    try {
      await invoke("update_user_profile", { ...profile });
      setUserProfile(profile);
      return true;
    } catch (error) {
      console.error("Failed to save user profile:", error);
      return false;
    }
  };

  const completeOnboarding = async () => {
    try {
      await invoke("complete_onboarding");
      return true;
    } catch (error) {
      console.error("Failed to save user profile:", error);
      return false;
    }
  };

  const handleStepComplete = async (step: OnboardingStep) => {
    switch (step) {
      case OnboardingStep.PROFILE:
        setCurrentStep(OnboardingStep.PERMISSIONS);
        break;
      case OnboardingStep.PERMISSIONS:
        setCurrentStep(OnboardingStep.COMPLETION);
        break;
      case OnboardingStep.COMPLETION:
        await completeOnboarding();
        await getCurrentWindow().hide();
        break;
    }
  };

  return (
    <div className="flex flex-col bg-background">
      <div
        className="h-[30px] flex items-center border-b border-neutral-800"
        data-tauri-drag-region
      ></div>
      <div className="dark min-h-screen bg-background flex justify-center">
        <main className="flex-grow">
          <div className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
            {/* Step content */}
            <Card className="bg-neutral-900 shadow overflow-hidden rounded-lg p-6 border-neutral-800">
              {currentStep === OnboardingStep.PROFILE && (
                <ProfileSetup
                  initialProfile={userProfile}
                  onComplete={(profile) => {
                    saveUserProfile(profile).then(() => {
                      handleStepComplete(OnboardingStep.PROFILE);
                    });
                  }}
                />
              )}

              {currentStep === OnboardingStep.PERMISSIONS && (
                <PermissionsSetup
                  onPermissionGranted={() => {
                    setIsPermissionGranted(true);
                    handleStepComplete(OnboardingStep.PERMISSIONS);
                  }}
                />
              )}

              {currentStep === OnboardingStep.COMPLETION && (
                <CompletionStep
                  profile={userProfile}
                  onComplete={() =>
                    handleStepComplete(OnboardingStep.COMPLETION)
                  }
                />
              )}
            </Card>
          </div>
        </main>
      </div>
    </div>
  );
};
