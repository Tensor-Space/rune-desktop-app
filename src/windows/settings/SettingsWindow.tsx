import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Shield,
  Keyboard,
  Mic,
  ArrowRight,
  Check,
  CheckCircle2,
} from "lucide-react";
import { Settings } from "./types";
import { Button } from "@/components/ui/button";
import { Permissions } from "./pages/Permissions";
import { Audio } from "./pages/Audio";
import { Shortcuts } from "./pages/Shortcuts";

const steps = [
  { id: "permissions", title: "Permissions", icon: Shield },
  { id: "microphone", title: "Microphone", icon: Mic },
  { id: "shortcuts", title: "Shortcuts", icon: Keyboard },
];

type StepId = (typeof steps)[number]["id"];

export const SettingsWindow = () => {
  const [currentStep, setCurrentStep] = useState<StepId>("permissions");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [completedSteps, setCompletedSteps] = useState<StepId[]>([]);
  const [_settings, setSettings] = useState<Settings | null>(null);
  const [isOnboardingComplete, setIsOnboardingComplete] = useState(false);

  useEffect(() => {
    // Prevent scrolling on body and html
    document.body.style.overflow = "hidden";
    document.documentElement.style.overflow = "hidden";

    return () => {
      // Cleanup
      document.body.style.overflow = "";
      document.documentElement.style.overflow = "";
    };
  }, []);

  useEffect(() => {
    const initializeSettings = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        setSettings(settings);
        const completed: StepId[] = [];

        const accessibilityPermission = await invoke<boolean>(
          "check_accessibility_permissions",
        );
        if (accessibilityPermission) {
          completed.push("permissions");
        }
        if (settings.audio.default_device) {
          completed.push("microphone");
        }
        if (settings.shortcuts.record_key) {
          completed.push("shortcuts");
        }

        setCompletedSteps(completed);

        if (completed.length === steps.length) {
          setCurrentStep("shortcuts");
          setIsOnboardingComplete(true);
        } else {
          const firstUncompleted = steps.find(
            (step) => !completed.includes(step.id),
          );
          if (firstUncompleted) {
            setCurrentStep(firstUncompleted.id);
          }
        }
      } catch (error) {
        setError("Failed to initialize settings");
        console.error(error);
      } finally {
        setIsLoading(false);
      }
    };

    initializeSettings();
  }, []);

  const currentStepIndex = steps.findIndex((step) => step.id === currentStep);

  const markStepComplete = (stepId: StepId) => {
    if (!completedSteps.includes(stepId)) {
      setCompletedSteps([...completedSteps, stepId]);
    }
  };

  const moveToNextStep = () => {
    markStepComplete(currentStep);
    if (currentStepIndex < steps.length - 1) {
      setCurrentStep(steps[currentStepIndex + 1].id);
    } else {
      setIsOnboardingComplete(true);
    }
  };

  const moveToPreviousStep = () => {
    if (currentStepIndex > 0) {
      setCurrentStep(steps[currentStepIndex - 1].id);
    }
  };

  const handleStepClick = (stepId: StepId) => {
    setCurrentStep(stepId);
  };

  const finishOnboarding = async () => {
    try {
      await invoke("set_onboarding_complete", { complete: true });
      setIsOnboardingComplete(true);
    } catch (error) {
      console.error(error);
    }
  };

  const renderStep = () => {
    switch (currentStep) {
      case "permissions":
        return (
          <Permissions
            onComplete={() => markStepComplete("permissions")}
            isStepComplete={completedSteps.includes("permissions")}
          />
        );
      case "microphone":
        return (
          <Audio
            onComplete={() => markStepComplete("microphone")}
            isStepComplete={completedSteps.includes("microphone")}
          />
        );
      case "shortcuts":
        return (
          <Shortcuts
            onComplete={() => markStepComplete("shortcuts")}
            isStepComplete={completedSteps.includes("shortcuts")}
          />
        );
      default:
        return null;
    }
  };

  if (isLoading) {
    return (
      <div className="dark flex h-screen items-center justify-center bg-background">
        <div className="text-lg text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-screen bg-background text-foreground overflow-hidden"
      data-tauri-drag-region
    >
      {/* Main container with fixed height and proper overflow behavior */}
      <div className="flex flex-col h-full p-6 container mx-auto">
        {/* Header */}
        <div className="mb-10 text-center" data-tauri-drag-region>
          <h1 className="text-2xl font-medium">Rune Config & Settings</h1>
        </div>

        {/* Stepper */}
        <div className="flex justify-center mb-12">
          <div className="flex items-center">
            {steps.map((step, index) => (
              <div key={step.id} className="flex items-center">
                <button
                  onClick={() => handleStepClick(step.id)}
                  className={`
                  flex items-center justify-center w-10 h-10 rounded-full border-2
                  transition-colors duration-200
                  ${
                    step.id === currentStep
                      ? "border-primary bg-primary text-primary-foreground"
                      : completedSteps.includes(step.id)
                        ? "border-neutral-800 bg-primary/10 text-primary hover:bg-primary/20"
                        : "border-neutral-800/30 text-muted-foreground hover:border-muted-foreground/50"
                  }
                `}
                >
                  {completedSteps.includes(step.id) ? (
                    <Check className="h-5 w-5" />
                  ) : (
                    <step.icon className="h-5 w-5" />
                  )}
                </button>

                {/* Step name */}
                <div className="absolute mt-16 text-xs w-20 -ml-5 text-center">
                  {step.title}
                </div>

                {index < steps.length - 1 && (
                  <div
                    className={`w-32 h-[2px] ${
                      completedSteps.includes(step.id)
                        ? "bg-neutral-800"
                        : "bg-neutral-800/30"
                    }`}
                  />
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Error Message */}
        {error && (
          <div className="text-destructive text-center mb-4">{error}</div>
        )}

        {/* Current Step Content */}
        <div className="flex-1 overflow-auto mt-6 container">
          {renderStep()}
        </div>

        {/* Navigation Buttons */}
        <div className="flex justify-between pt-4 mt-auto">
          <Button
            variant="outline"
            onClick={moveToPreviousStep}
            disabled={currentStepIndex === 0}
          >
            Back
          </Button>

          {isOnboardingComplete ? (
            <Button
              variant="default"
              onClick={() => {
                alert("Setup complete!");
              }}
            >
              <CheckCircle2 className="h-4 w-4" />
              Done (close)
            </Button>
          ) : (
            <Button
              onClick={
                currentStepIndex === steps.length - 1
                  ? finishOnboarding
                  : moveToNextStep
              }
              disabled={!completedSteps.includes(currentStep)}
            >
              {currentStepIndex === steps.length - 1 ? (
                "Finish"
              ) : (
                <>
                  Next <ArrowRight className="ml-2 h-4 w-4" />
                </>
              )}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
};
