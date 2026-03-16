import { lazy, Suspense } from "react";
import { useWindowLabel } from "./features/layout/hooks/useWindowLabel";
import { AppShell } from "./app-shell";

const AboutView = lazy(() =>
  import("./features/about/components/AboutView").then((module) => ({
    default: module.AboutView,
  })),
);

export function AppRouter() {
  const windowLabel = useWindowLabel();
  if (windowLabel === "about") {
    return (
      <Suspense fallback={null}>
        <AboutView />
      </Suspense>
    );
  }
  return <AppShell />;
}

export default AppRouter;
