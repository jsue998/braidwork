import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
export async function chooseDirectory(): Promise<string | null> {
  const path = await open({
    directory: true,
    multiple: false,
    title: "Choose a project folder",
  });
  return typeof path === "string" ? path : null;
}
export async function chooseFiles(multiple: boolean): Promise<string[]> {
  const paths = await open({
    directory: false,
    multiple,
    title: multiple ? "Choose context files (UTF-8)" : "Choose a result file",
  });
  return paths === null ? [] : typeof paths === "string" ? [paths] : paths;
}
export const copyText = (text: string) => writeText(text);
