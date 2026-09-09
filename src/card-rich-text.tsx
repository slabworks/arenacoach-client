import CardName from "./card-name";
import type { CardRef } from "./matches";

export default function CardRichText({
  text,
  cards,
}: {
  text: string;
  cards: Record<string, CardRef>;
}) {
  const names = Object.entries(cards)
    .map(([id, card]) => ({
      id: Number(id),
      name: card.name,
    }))
    .filter((card) => card.name.length > 0)
    .sort((left, right) => right.name.length - left.name.length);

  if (names.length === 0) {
    return text;
  }

  const pattern = new RegExp(
    `(${names.map((card) => escapeRegExp(card.name)).join("|")})`,
    "g",
  );
  const byName = new Map(names.map((card) => [card.name, card.id]));

  return (
    <>
      {text.split(pattern).map((part, index) => {
        const grpId = byName.get(part);

        if (grpId === undefined) {
          return <span key={index}>{part}</span>;
        }

        return <CardName key={index} grpId={grpId} label={part} />;
      })}
    </>
  );
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
