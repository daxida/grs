import classNames from "classnames";
import RepoButton from "./RepoButton";
import { Theme } from "./theme";
import { to_monotonic } from "./../pkg/grs_wasm";
import { Source } from "./App";

export default function Header({
  source,
  theme,
  onChangeTheme,
  onSourceChanged,
}: {
  source: Source | null
  theme: Theme;
  onChangeTheme: (theme: Theme) => void;
  onSourceChanged(source: string): void;
}) {
  return (
    <div
      className={classNames(
        "w-full",
        "flex",
        "justify-between",
        "pl-5",
        "sm:pl-1",
        "pr-4",
        "lg:pr-6",
        "z-10",
        "top-0",
        "left-0",
        "-mb-px",
        "antialiased",
        "border-b",
        "border-gray-200",
        "dark:border-b-radiate",
        "dark:bg-galaxy",
      )}
    >
      <div className="text-3xl"> GRS </div>
      <div className="flex items-center min-w-0">
        <ToMonotonicButton source={source} onSourceChanged={onSourceChanged} />
        <Divider />
        <RepoButton />
        <Divider />
        <ThemeButton theme={theme} onChangeTheme={onChangeTheme} />
      </div>
    </div>
  );
};

function ThemeButton({
  theme,
  onChangeTheme,
}: {
  theme: Theme;
  onChangeTheme: (theme: Theme) => void;
}) {
  return (
    <button
      onClick={() => onChangeTheme(theme === "light" ? "dark" : "light")}
    >
      {theme === "light" ? "Dark Mode" : "Light Mode"}
    </button>
  );
}

function ToMonotonicButton({
  source,
  onSourceChanged,
}: {
  source: Source | null;
  onSourceChanged: (source: string) => void;
}) {
  const handleClick = () => {
    if (source === null) return;
    const monotonicText = to_monotonic(source.text);
    onSourceChanged(monotonicText);
  };
  return <button onClick={handleClick}>To Monotonic</button>;
}

function Divider() {
  return (
    <div className="hidden sm:block mx-6 lg:mx-4 w-px h-8 bg-gray-200 dark:bg-gray-700" />
  );
}
