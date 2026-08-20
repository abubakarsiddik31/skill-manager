import { projectsApi } from "./projects";
import { skillsApi } from "./skills";

/** Single place the frontend talks to the Rust command layer. */
export const api = { ...skillsApi, ...projectsApi };
