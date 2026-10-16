import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => cleanup());

// Konva necesita un <canvas> real; todos los tests de UI usan el doble de __mocks__.
vi.mock("../ui/annotator/AnnotatorCanvas");
