/**
 * Input validation: supported languages and filesystem paths.
 */
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { InvalidLanguageError, InvalidPathError, IoError, toMessage } from "./errors.js";

/** Languages the translator can target. */
export const SUPPORTED_LANGUAGES = ["en", "fr"] as const;
export type TargetLanguage = (typeof SUPPORTED_LANGUAGES)[number];

/** Validate and narrow an arbitrary string to a {@link TargetLanguage}. */
export function assertLanguage(value: string): TargetLanguage {
  const normalised = value.trim().toLowerCase();
  if ((SUPPORTED_LANGUAGES as readonly string[]).includes(normalised)) {
    return normalised as TargetLanguage;
  }
  throw new InvalidLanguageError(value, SUPPORTED_LANGUAGES);
}

export type PathKind = "file" | "directory";

/**
 * Resolve `target` to an absolute path and confirm it exists with the expected
 * kind. Throws {@link InvalidPathError} for missing/mistyped paths.
 */
export async function assertPath(
  target: string,
  expected: PathKind | "any" = "any",
): Promise<string> {
  if (!target || !target.trim()) {
    throw new InvalidPathError("A non-empty path is required.");
  }
  const absolute = path.resolve(target);
  let stat: Awaited<ReturnType<typeof fs.stat>>;
  try {
    stat = await fs.stat(absolute);
  } catch (err) {
    const code = (err as NodeJS.ErrnoException)?.code;
    if (code === "ENOENT") {
      throw new InvalidPathError(`Path does not exist: ${absolute}`, { cause: err });
    }
    throw new IoError(`Cannot access path ${absolute}: ${toMessage(err)}`, { cause: err });
  }
  if (expected === "file" && !stat.isFile()) {
    throw new InvalidPathError(`Expected a file but found a directory: ${absolute}`);
  }
  if (expected === "directory" && !stat.isDirectory()) {
    throw new InvalidPathError(`Expected a directory but found a file: ${absolute}`);
  }
  if (expected === "any" && !stat.isFile() && !stat.isDirectory()) {
    throw new InvalidPathError(`Path is neither a file nor a directory: ${absolute}`);
  }
  return absolute;
}
