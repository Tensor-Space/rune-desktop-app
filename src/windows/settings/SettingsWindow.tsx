import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Shield, Keyboard, Volume2, Brain } from "lucide-react";
import { Settings } from "./types";
import { AIModels } from "./pages/AIModels";
import { Shortcuts } from "./pages/Shortcuts";
import { Audio } from "./pages/Audio";
import { Permissions } from "./pages/Permissions";
import { History } from "./pages/History";

const sidebarItems = [
  { id: "permissions", label: "Permissions", icon: Shield },
  { id: "shortcuts", label: "Shortcuts", icon: Keyboard },
  { id: "audio", label: "Audio", icon: Volume2 },
  { id: "ai-models", label: "AI Models", icon: Brain },
  { id: "history", label: "History", icon: Keyboard },
] as const;

type PageId = (typeof sidebarItems)[number]["id"];

export const SettingsWindow = () => {
  const [activePage, setActivePage] = useState<PageId>("permissions");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initializeSettings = async () => {
      try {
        await invoke<Settings>("get_settings");
        setError(null);
      } catch (error) {
        setError("Failed to initialize settings");
        console.error("Settings initialization error:", error);
      } finally {
        setIsLoading(false);
      }
    };

    initializeSettings();
  }, []);

  const renderPage = () => {
    switch (activePage) {
      case "permissions":
        return <Permissions />;
      case "history":
        return <History />;
      case "shortcuts":
        return <Shortcuts />;
      case "audio":
        return <Audio />;
      case "ai-models":
        return <AIModels />;
    }
  };

  if (isLoading) {
    return (
      <div className="dark flex h-screen items-center justify-center bg-background">
        <div className="text-lg text-muted-foreground">Loading settings...</div>
      </div>
    );
  }

  return (
    <div
      className="dark flex h-screen bg-background text-foreground"
      data-tauri-drag-region
    >
      {/* Sidebar */}
      <div
        className="w-64 border-r border-border bg-card"
        data-tauri-drag-region
      >
        <div className="p-6" data-tauri-drag-region></div>
        <nav className="space-y-2 p-4">
          {sidebarItems.map(({ id, label, icon: Icon }) => (
            <Button
              key={id}
              variant="ghost"
              className={cn(
                "w-full justify-start gap-2 hover:bg-accent/50 hover:cursor-pointer text-foreground/70",
                activePage === id &&
                  "bg-accent hover:bg-accent text-foreground",
              )}
              onClick={() => setActivePage(id)}
            >
              <Icon className="h-4 w-4" />
              {label}
            </Button>
          ))}
        </nav>
      </div>

      {/* Main Content */}
      <ScrollArea className="flex-1">
        <div className="flex flex-col h-screen max-w-3xl py-8 px-4 mx-auto">
          {error && (
            <div className="text-destructive text-center mb-4">{error}</div>
          )}
          <div className="flex-1">{renderPage()}</div>
          <div className="text-center text-sm text-muted-foreground/40 mt-8">
            <p>Rune v1.0.0</p>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
};
