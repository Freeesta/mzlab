// The page picks elements by selector and then uses them as what they are (el.querySelector('[data-a="fit"]').onclick = ...): in a non-strict
// project the selector overloads return any instead of Element. Not included in the worker project (no DOM there).
interface ParentNode {
  querySelector(selectors: string): any;
  querySelectorAll(selectors: string): NodeListOf<any>;
}
