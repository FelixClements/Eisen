import { writable } from 'svelte/store';

export const search = writable('');
export const showSearch = writable(false);
export const syncMessage = writable('');
