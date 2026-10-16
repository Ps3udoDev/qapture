import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import App from "./App";

it("muestra la marca Qapture", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "Qapture" })).toBeInTheDocument();
});
