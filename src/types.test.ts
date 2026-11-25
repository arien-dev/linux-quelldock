import { describe, expect, it } from "vitest";
import { providerLabel } from "./types";

describe("provider labels", () => {
  it("covers every supported package source", () => {
    expect(Object.keys(providerLabel)).toEqual(["app_image", "flatpak", "aur", "pacman", "apt", "dnf", "zypper"]);
    expect(providerLabel.app_image).toBe("AppImage");
  });
});
