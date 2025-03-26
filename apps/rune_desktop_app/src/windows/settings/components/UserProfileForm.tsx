import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, CheckCircle2 } from "lucide-react";
import { Settings } from "../types";

interface UserProfileFormProps {
  onProfileSet?: (hasProfile: boolean) => void;
}

export const UserProfileForm = ({ onProfileSet }: UserProfileFormProps) => {
  const [name, setName] = useState<string>("");
  const [email, setEmail] = useState<string>("");
  const [about, setAbout] = useState<string>("");
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<boolean>(false);
  const [, setHasProfile] = useState<boolean>(false);

  useEffect(() => {
    const loadUserProfile = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        if (settings.user_profile) {
          setName(settings.user_profile.name || "");
          setEmail(settings.user_profile.email || "");
          setAbout(settings.user_profile.about || "");

          const profileComplete = !!(
            settings.user_profile.name || settings.user_profile.email
          );
          setHasProfile(profileComplete);
          if (profileComplete) {
            onProfileSet?.(true);
          }
        }
      } catch (error) {
        setError("Failed to load user profile");
        console.error("Profile load error:", error);
      }
    };

    loadUserProfile();
  }, [onProfileSet]);

  const handleSaveProfile = async () => {
    setIsSaving(true);
    setError(null);
    setSuccess(false);

    try {
      // Basic email validation
      if (email && !email.includes("@")) {
        setError("Please enter a valid email address");
        setIsSaving(false);
        return;
      }

      await invoke("update_user_profile", {
        name: name.trim() || null,
        email: email.trim() || null,
        description: about.trim() || null,
      });

      setSuccess(true);
      setHasProfile(!!(name || email));
      onProfileSet?.(!!(name || email));
    } catch (error) {
      setError(`Failed to save profile: ${error}`);
      console.error("Failed to save profile:", error);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>Your Profile</CardTitle>
        <CardDescription>
          Tell us a bit about yourself to personalize your experience
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {success && (
          <Alert className="bg-green-500/10 border-green-500 text-green-500">
            <CheckCircle2 className="h-4 w-4" />
            <AlertDescription>Profile saved successfully</AlertDescription>
          </Alert>
        )}

        <div className="space-y-4">
          <div className="space-y-2">
            <label htmlFor="name" className="text-sm font-medium">
              Name
            </label>
            <Input
              id="name"
              placeholder="Your name"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setSuccess(false);
              }}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="email" className="text-sm font-medium">
              Email
            </label>
            <Input
              id="email"
              type="email"
              placeholder="Your email address"
              value={email}
              onChange={(e) => {
                setEmail(e.target.value);
                setSuccess(false);
              }}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="description" className="text-sm font-medium">
              About you (optional)
            </label>
            <Input
              id="description"
              placeholder="Brief description about yourself"
              value={about}
              onChange={(e) => {
                setAbout(e.target.value);
                setSuccess(false);
              }}
            />
            <p className="text-xs text-muted-foreground">
              This information helps us provide more personalized assistance
            </p>
          </div>
        </div>
      </CardContent>
      <CardFooter>
        <Button
          onClick={handleSaveProfile}
          disabled={isSaving}
          className="w-full"
        >
          {isSaving ? "Saving..." : "Save Profile"}
        </Button>
      </CardFooter>
    </Card>
  );
};
