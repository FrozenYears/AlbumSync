// 极简 hash router：用 URL hash 切页面，零依赖

import { useEffect, useState } from "react";

export type Route = "onboarding" | "sync" | "history" | "trash" | "settings";

export function useRoute(): [Route, (r: Route) => void] {
  const parse = (): Route => {
    const h = window.location.hash.replace(/^#\/?/, "") || "sync";
    return (["onboarding", "sync", "history", "trash", "settings"].includes(h)
      ? h
      : "sync") as Route;
  };
  const [route, setRoute] = useState<Route>(parse);
  useEffect(() => {
    const onHash = () => setRoute(parse());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  const navigate = (r: Route) => {
    window.location.hash = `/${r}`;
  };
  return [route, navigate];
}
