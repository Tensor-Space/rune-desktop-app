import { Button } from "@/components/ui/button";

interface UserProfile {
  name: string;
  email: string;
  about: string;
}

interface CompletionStepProps {
  profile: UserProfile;
  onComplete: () => void;
}

export const CompletionStep = ({
  profile,
  onComplete,
}: CompletionStepProps) => {
  return (
    <div className="text-center">
      <svg
        className="mx-auto h-16 w-16 text-green-500"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>

      <h2 className="mt-4 text-2xl font-bold">All Set, {profile.name}!</h2>

      <p className="mt-2 text-neutral-400">
        Your profile has been set up and all necessary permissions have been
        granted. You're ready to start using RuneApp!
      </p>

      <div className="mt-8">
        <Button onClick={onComplete}>Get Started</Button>
      </div>
    </div>
  );
};
