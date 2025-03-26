import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";

interface UserProfile {
  name: string;
  email: string;
  about: string;
}

interface ProfileSetupProps {
  initialProfile: UserProfile;
  onComplete: (profile: UserProfile) => void;
}

export const ProfileSetup = ({
  initialProfile,
  onComplete,
}: ProfileSetupProps) => {
  const [profile, setProfile] = useState<UserProfile>(initialProfile);
  const [errors, setErrors] = useState<
    Partial<Record<keyof UserProfile, string>>
  >({});

  const validateForm = (): boolean => {
    const newErrors: Partial<Record<keyof UserProfile, string>> = {};

    if (!profile.name.trim()) {
      newErrors.name = "Name is required";
    }

    if (!profile.email.trim()) {
      newErrors.email = "Email is required";
    } else if (!/^\S+@\S+\.\S+$/.test(profile.email)) {
      newErrors.email = "Email is invalid";
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validateForm()) {
      onComplete(profile);
    }
  };

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">Tell us about yourself</h2>
      <p className="text-neutral-400 mb-6">
        We'll use this information to personalize your experience.
      </p>

      <form onSubmit={handleSubmit}>
        <div className="space-y-6">
          <div>
            <Label
              htmlFor="name"
              className="block text-sm font-medium text-neutral-300"
            >
              Name
            </Label>
            <Input
              id="name"
              className={`mt-1 bg-neutral-800 border-neutral-700 ${
                errors.name ? "border-red-500" : ""
              }`}
              value={profile.name}
              onChange={(e) => setProfile({ ...profile, name: e.target.value })}
              placeholder="Your name"
            />
            {errors.name && (
              <p className="mt-1 text-sm text-red-500">{errors.name}</p>
            )}
          </div>

          <div>
            <Label
              htmlFor="email"
              className="block text-sm font-medium text-neutral-300"
            >
              Email
            </Label>
            <Input
              type="email"
              id="email"
              className={`mt-1 bg-neutral-800 border-neutral-700 ${
                errors.email ? "border-red-500" : ""
              }`}
              value={profile.email}
              onChange={(e) =>
                setProfile({ ...profile, email: e.target.value })
              }
              placeholder="your.email@example.com"
            />
            {errors.email && (
              <p className="mt-1 text-sm text-red-500">{errors.email}</p>
            )}
          </div>

          <div>
            <Label
              htmlFor="about"
              className="block text-sm font-medium text-neutral-300"
            >
              About
            </Label>
            <Textarea
              id="about"
              rows={3}
              value={profile.about}
              onChange={(e) =>
                setProfile({ ...profile, about: e.target.value })
              }
              placeholder="Tell us a bit about yourself..."
            />
          </div>

          <div className="flex justify-end">
            <Button type="submit" className="">
              Next
            </Button>
          </div>
        </div>
      </form>
    </div>
  );
};
