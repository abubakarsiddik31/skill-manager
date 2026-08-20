import type { ProjectInfo } from "./project";

export type View = { kind: "global" } | { kind: "project"; project: ProjectInfo };
