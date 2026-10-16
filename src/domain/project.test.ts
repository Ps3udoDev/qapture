import { describe, expect, it } from "vitest";
import { emptyMeta, newFieldInput, normalizeFieldInput, validateFieldInputs, validateProjectMeta } from "./project";

describe("validateProjectMeta", () => {
  it("exige nombre y fechas coherentes", () => {
    expect(validateProjectMeta({ ...emptyMeta(), name: " " })).toEqual({ name: "«Nombre del proyecto» es obligatorio." });
    expect(validateProjectMeta({ ...emptyMeta(), name: "Ok", startDate: "2026-13-01" })).toEqual({
      startDate: "«Fecha de inicio» debe ser una fecha válida (AAAA-MM-DD).",
    });
    expect(validateProjectMeta({ ...emptyMeta(), name: "Ok", startDate: "2026-11-01", endDate: "2026-10-01" })).toEqual({
      endDate: "La fecha de fin no puede ser anterior a la de inicio.",
    });
  });
});

describe("validateFieldInputs", () => {
  it("rechaza anchos no enteros", () => {
    expect(validateFieldInputs([{ ...newFieldInput(), label: "Ancho", width: 12.5 }])).toEqual({
      "0": "El ancho de «Ancho» debe estar entre 8 y 100.",
    });
  });

  it("detecta nombres vacíos, repetidos, listas sin opciones y anchos inválidos", () => {
    const errors = validateFieldInputs([
      { ...newFieldInput(), label: "Entorno" },
      { ...newFieldInput(), label: "entorno " },
      { ...newFieldInput(), label: "" },
      { ...newFieldInput(), label: "Severidad", type: "list", options: [" ", ""] },
      { ...newFieldInput(), label: "Ancho", width: 4 },
      { ...newFieldInput(), label: "Entorno", archived: true },
    ]);
    expect(errors).toEqual({
      "1": "La columna «entorno» está repetida.",
      "2": "La columna 3 necesita un nombre.",
      "3": "La lista «Severidad» necesita al menos una opción.",
      "4": "El ancho de «Ancho» debe estar entre 8 y 100.",
    });
  });

  it("replica los límites de Rust y su orden de comprobación", () => {
    const long = "x".repeat(101);
    const many = Array.from({ length: 51 }, (_, i) => `o${i}`);
    const errors = validateFieldInputs([
      { ...newFieldInput(), label: long },
      { ...newFieldInput(), label: "Sev", type: "list", options: ["ok", long] },
      { ...newFieldInput(), label: "Muchas", type: "list", options: many },
      { ...newFieldInput(), label: "Ambas", type: "list", options: [], width: 4 },
      { ...newFieldInput(), label: "Cincuenta", type: "list", options: many.slice(0, 50) },
    ]);
    expect(errors).toEqual({
      "0": "«Nombre de la columna» supera 100 caracteres.",
      "1": "Una opción de «Sev» supera 100 caracteres.",
      "2": "La lista «Muchas» admite como máximo 50 opciones.",
      "3": "El ancho de «Ambas» debe estar entre 8 y 100.",
    });
  });

  it("normaliza opciones solo para listas", () => {
    expect(normalizeFieldInput({ ...newFieldInput(), label: " Sev ", type: "list", options: [" Alta", "Alta", "", "Baja"] })).toMatchObject({
      label: "Sev",
      options: ["Alta", "Baja"],
    });
    expect(normalizeFieldInput({ ...newFieldInput(), label: "X", type: "text", options: ["a"] }).options).toEqual([]);
  });
});
