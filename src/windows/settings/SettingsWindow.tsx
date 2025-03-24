import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Shield,
  Keyboard,
  Mic,
  CheckCircle2,
  Settings as SettingsIcon,
} from "lucide-react";
import { Settings } from "./types";
import { Permissions } from "./pages/Accessibility";
import { Audio } from "./pages/Microphone";
import { Shortcuts } from "./pages/Shortcuts";
import { cn } from "@/lib/utils";

const sections = [
  { id: "permissions", title: "Permissions", icon: Shield },
  { id: "microphone", title: "Microphone", icon: Mic },
  { id: "shortcuts", title: "Shortcuts", icon: Keyboard },
];

type SectionId = (typeof sections)[number]["id"];

export const SettingsWindow = () => {
  const [currentSection, setCurrentSection] =
    useState<SectionId>("permissions");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [completedSections, setCompletedSections] = useState<SectionId[]>([]);
  const [_settings, setSettings] = useState<Settings | null>(null);
  const [, setIsOnboardingComplete] = useState(false);
  const pollingIntervalRef = useRef<number | null>(null);

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

  // Function to fetch settings and update state
  const fetchSettings = async () => {
    try {
      const settings = await invoke<Settings>("get_settings");
      setSettings(settings);
      const completed: SectionId[] = [];

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

      setCompletedSections(completed);

      if (completed.length === sections.length) {
        setIsOnboardingComplete(true);
      } else if (isLoading) {
        const firstUncompleted = sections.find(
          (section) => !completed.includes(section.id),
        );
        if (firstUncompleted) {
          setCurrentSection(firstUncompleted.id);
        }
      }
    } catch (error) {
      setError("Failed to fetch settings");
      console.error(error);
    } finally {
      if (isLoading) {
        setIsLoading(false);
      }
    }
  };

  useEffect(() => {
    // Initial fetch
    fetchSettings();

    // Set up polling every 2 seconds
    const intervalId = window.setInterval(() => {
      fetchSettings();
    }, 2000);

    pollingIntervalRef.current = intervalId;

    // Cleanup interval on component unmount
    return () => {
      if (pollingIntervalRef.current !== null) {
        clearInterval(pollingIntervalRef.current);
      }
    };
  }, []);

  const markSectionComplete = (sectionId: SectionId) => {
    if (!completedSections.includes(sectionId)) {
      setCompletedSections([...completedSections, sectionId]);
    }
  };

  const handleSectionClick = (sectionId: SectionId) => {
    setCurrentSection(sectionId);
  };

  const renderSection = () => {
    switch (currentSection) {
      case "permissions":
        return (
          <Permissions
            onComplete={() => markSectionComplete("permissions")}
            isStepComplete={completedSections.includes("permissions")}
          />
        );
      case "microphone":
        return (
          <Audio
            onComplete={() => markSectionComplete("microphone")}
            isStepComplete={completedSections.includes("microphone")}
          />
        );
      case "shortcuts":
        return (
          <Shortcuts
            onComplete={() => markSectionComplete("shortcuts")}
            isStepComplete={completedSections.includes("shortcuts")}
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
      className="flex h-screen bg-background text-foreground overflow-hidden"
      data-tauri-drag-region
    >
      {/* Sidebar */}
      <div className="w-60 h-full border-r border-neutral-800 bg-muted/30">
        {/* App title */}
        <div
          className="p-4 h-14 flex items-center border-b border-neutral-800"
          data-tauri-drag-region
        >
          <SettingsIcon className="h-5 w-5 mr-2" />
          <h1 className="text-md font-medium">Settings</h1>
        </div>

        {/* Navigation */}
        <nav className="p-2">
          <ul className="space-y-1">
            {sections.map((section) => (
              <li key={section.id}>
                <button
                  onClick={() => handleSectionClick(section.id)}
                  className={cn(
                    "w-full flex items-center px-3 py-2 rounded-md text-sm transition-colors",
                    "hover:bg-primary/10",
                    currentSection === section.id
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-muted-foreground",
                  )}
                >
                  <section.icon className="h-4 w-4 mr-2" />
                  <span>{section.title}</span>
                  {completedSections.includes(section.id) && (
                    <CheckCircle2 className="h-3.5 w-3.5 ml-auto text-primary" />
                  )}
                </button>
              </li>
            ))}
          </ul>
        </nav>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        {/* Header */}
        <div
          className="h-14 border-b border-neutral-800 flex items-center px-6"
          data-tauri-drag-region
        >
          <h2 className="text-lg font-medium">
            {sections.find((s) => s.id === currentSection)?.title}
          </h2>
        </div>

        {/* Content area */}
        <div className="flex-1 overflow-auto p-6">
          {error && (
            <div className="text-destructive text-center mb-4">{error}</div>
          )}
          {renderSection()}
        </div>
      </div>
    </div>
  );
};
