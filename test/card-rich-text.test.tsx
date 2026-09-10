/// <reference lib="dom" />

import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import CardRichText from "../src/card-rich-text";

describe("CardRichText", () => {
  test("renders plain text when no card names are available", () => {
    render(<CardRichText text="Keep the land." cards={{}} />);

    expect(screen.getByText("Keep the land.")).toBeInTheDocument();
    expect(document.querySelector(".card-name")).toBeNull();
  });

  test("turns known card names into hoverable labels", () => {
    render(
      <CardRichText
        text="Cast Lightning Bolt on the bird."
        cards={{ "1": { name: "Lightning Bolt" } }}
      />,
    );

    expect(screen.getByText("Lightning Bolt")).toHaveClass("card-name");
    expect(screen.getByText(/Cast/)).toBeInTheDocument();
    expect(screen.getByText(/on the bird/)).toBeInTheDocument();
  });

  test("prefers the longer card name when one name contains another", () => {
    render(
      <CardRichText
        text="I played Lightning Bolt."
        cards={{
          "1": { name: "Lightning" },
          "2": { name: "Lightning Bolt" },
        }}
      />,
    );

    expect(screen.getByText("Lightning Bolt")).toHaveClass("card-name");
    expect(screen.queryByText("Lightning", { exact: true })).toBeNull();
  });

  test("matches names that include regex characters", () => {
    render(
      <CardRichText
        text="Fire (token) won the race."
        cards={{ "8": { name: "Fire (token)" } }}
      />,
    );

    expect(screen.getByText("Fire (token)")).toHaveClass("card-name");
  });
});
