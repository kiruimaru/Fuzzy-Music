// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { RuntimeSetup } from "./RuntimeSetup";
import { api } from "./api";

vi.mock("./api", async original => ({
  ...await original<typeof import("./api")>(),
  api: { runtimeStatus: vi.fn(), installRuntime: vi.fn() },
}));
afterEach(() => { cleanup(); vi.resetAllMocks(); });
const missing = { supported: true, ytDlp: false, mpv: false, ffmpeg: false, canInstallPackages: true };

test("system packages require explicit consent and successful setup removes first-run panel", async () => {
  vi.mocked(api.runtimeStatus).mockResolvedValue(missing);
  vi.mocked(api.installRuntime).mockResolvedValue({ ...missing, ytDlp: true, mpv: true });
  render(<RuntimeSetup />);
  const button = await screen.findByRole("button", { name: "Install dependencies" });
  fireEvent.click(screen.getByLabelText("Install system packages (administrator approval)"));
  fireEvent.click(screen.getByLabelText("Include ffmpeg"));
  fireEvent.click(button);
  await waitFor(() => expect(api.installRuntime).toHaveBeenCalledWith(true, true));
  await waitFor(() => expect(screen.queryByLabelText("Runtime setup")).toBeNull());
});

test("download-only setup does not authorize system installation; failure permits retry", async () => {
  vi.mocked(api.runtimeStatus).mockResolvedValue(missing);
  vi.mocked(api.installRuntime).mockRejectedValue({ message: "Runtime download failed.", code: "source_unavailable", retryable: true });
  render(<RuntimeSetup />);
  fireEvent.click(await screen.findByRole("button", { name: "Install dependencies" }));
  await screen.findByRole("alert");
  expect(api.installRuntime).toHaveBeenCalledWith(false, false);
  expect(screen.getByRole("button", { name: "Install dependencies" }).hasAttribute("disabled")).toBe(false);
});

test("settings keeps ready dependency status available", async () => {
  vi.mocked(api.runtimeStatus).mockResolvedValue({ ...missing, ytDlp: true, mpv: true, ffmpeg: true });
  render(<RuntimeSetup settings />);
  expect(await screen.findByLabelText("Runtime setup")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Install dependencies" })).toBeNull();
});
