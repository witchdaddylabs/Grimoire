export type PalaceItem = {
  id: string;
  title: string;
  kind: "character" | "location" | "chapter";
  path: string;
  body: string;
};

export const palaceItems: PalaceItem[] = [
  {
    id: "mara",
    title: "Mara Thorne",
    kind: "character",
    path: "The Novel / Characters / Protagonists / Main Cast",
    body:
      "Mara speaks like someone who learned early that every word can be used against her. When she is angry, her sentences become shorter. When she is afraid, she becomes polite.",
  },
  {
    id: "vel-ashen",
    title: "Vel Ashen",
    kind: "location",
    path: "The Novel / World / Cities / Northern Cities",
    body:
      "Vel Ashen smells of river silt, old stone, bridge smoke, and wet iron at dawn. The city is not romantic to Mara. It is familiar, dangerous, and useful.",
  },
  {
    id: "chapter-01",
    title: "Chapter 01: The Bell Beneath the River",
    kind: "chapter",
    path: "The Novel / Drafts / Act One / Opening Sequence",
    body:
      "The bell rang below the river before anyone in Vel Ashen admitted they could hear it. Mara counted the sound by instinct and kept walking.",
  },
];

export const retrievalSteps = [
  "Consulting the Palace",
  "Reading canon traces",
  "Checking slop wards",
  "Composing grounded answer",
];

