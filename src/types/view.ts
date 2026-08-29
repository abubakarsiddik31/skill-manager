import type { ProjectInfo } from "./project";

/** The main window shows one view at a time; "browse" is the
 *  collections browser that replaces the skill grid until exited. */
export type View =
  | { kind: "global" }
  | { kind: "project"; project: ProjectInfo }
  | { kind: "browse" };
