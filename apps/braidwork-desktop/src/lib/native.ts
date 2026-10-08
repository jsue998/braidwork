import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
export async function chooseDirectory(title: string): Promise<string | null> {
  const path = await open({
    directory: true,
    multiple: false,
    title,
  });
  return typeof path === "string" ? path : null;
}
export async function chooseFiles(multiple: boolean, title: string): Promise<string[]> {
  const paths = await open({
    directory: false,
    multiple,
    title,
  });
  return paths === null ? [] : typeof paths === "string" ? [paths] : paths;
}
export const copyText = (text: string) => writeText(text);
