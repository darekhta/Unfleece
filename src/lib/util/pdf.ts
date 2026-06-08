/** Standard page sizes in PDF points (1/72 inch). */
export const PAGE_SIZES = {
  a4: [595.28, 841.89] as [number, number],
  letter: [612, 792] as [number, number],
} satisfies Record<string, [number, number]>;
