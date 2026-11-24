// A view's filters and sort order, remembered between launches. Search text
// is not remembered: an old search silently hiding fonts is confusing.

import type { FilterState, SortState } from './query';

export class QueryState {
  filters = $state<FilterState>({});
  sort = $state<SortState>({ key: '', dir: 'asc' });
  /**
   * The order while a search is typed. Searching switches to best match
   * without touching the saved order, and a different pick made while
   * searching lasts until the search is cleared.
   */
  searchSort = $state<SortState | null>(null);

  constructor(
    private storageKey: string,
    defaultSort: SortState,
  ) {
    this.sort = defaultSort;
    try {
      const saved = JSON.parse(localStorage.getItem(storageKey) ?? 'null');
      if (saved?.filters && typeof saved.filters === 'object') this.filters = saved.filters;
      if (saved?.sort?.key) this.sort = saved.sort;
    } catch {
      /* storage unavailable or corrupt: defaults */
    }
  }

  save() {
    try {
      localStorage.setItem(this.storageKey, JSON.stringify({ filters: this.filters, sort: this.sort }));
    } catch {
      /* not persisted */
    }
  }

  setFilters(next: FilterState) {
    this.filters = next;
    this.save();
  }

  setSort(next: SortState) {
    this.sort = next;
    this.save();
  }

  /** The order in effect. */
  effective(searching: boolean): SortState {
    if (searching) return this.searchSort ?? { key: 'relevance', dir: 'desc' };
    return this.sort.key === 'relevance' ? { key: 'name', dir: 'asc' } : this.sort;
  }

  pick(next: SortState, searching: boolean) {
    if (searching) this.searchSort = next;
    else this.setSort(next);
  }

  /** Forget the order picked during a search, once it's cleared. */
  endSearch() {
    this.searchSort = null;
  }

  clear() {
    this.setFilters({});
  }
}
