/// <reference lib="dom" />
import { afterEach, expect, spyOn, test } from 'bun:test';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import * as images from '../src/card-image';
import CardName from '../src/card-name';

const spies: ReturnType<typeof spyOn>[] = [];
afterEach(() => { for (const spy of spies.splice(0)) spy.mockRestore(); });

test('a late card image does not reopen a preview after pointer departure', async () => {
    let resolve!: (value: string | null) => void;
    const request = new Promise<string | null>((done) => { resolve = done; });
    const spy = spyOn(images, 'loadCardImage').mockReturnValue(request);
    spies.push(spy);
    render(<CardName grpId={1} label="Mountain" />);
    const trigger = screen.getByRole('button', { name: 'Mountain' });
    fireEvent.mouseEnter(trigger);
    await waitFor(() => expect(spy).toHaveBeenCalled());
    fireEvent.mouseLeave(trigger);
    await act(async () => { resolve('https://example.com/card.jpg'); await request; });
    expect(screen.queryByRole('tooltip')).toBeNull();
});

test('keyboard users can inspect and dismiss a card preview', async () => {
    spies.push(spyOn(images, 'loadCardImage').mockResolvedValue('https://example.com/card.jpg'));
    render(<CardName grpId={1} label="Mountain" />);
    const trigger = screen.getByRole('button', { name: 'Mountain' });
    fireEvent.focus(trigger);
    await waitFor(() => expect(screen.getByRole('img', { name: 'Mountain' })).toBeInTheDocument());
    fireEvent.keyDown(trigger, { key: 'Escape' });
    expect(screen.queryByRole('tooltip')).toBeNull();
});
