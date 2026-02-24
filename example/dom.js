/**
 * Debounces a function so it only fires after `delay` ms of inactivity.
 * @param {Function} fn - The function to debounce.
 * @param {number} delay - Milliseconds to wait.
 * @returns {Function} The debounced function.
 */
function debounce(fn, delay) {
  let timer;
  return function (...args) {
    clearTimeout(timer);
    timer = setTimeout(() => fn.apply(this, args), delay);
  };
}

/**
 * Deep-clones a plain object or array using JSON round-trip.
 * @param {*} value - Value to clone.
 * @returns {*} A deep copy.
 */
function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

/**
 * Groups an array of objects by the value of a given key.
 * @param {Object[]} items - Array of objects.
 * @param {string} key - Property name to group by.
 * @returns {Object} Map from key value to array of items.
 */
function groupBy(items, key) {
  return items.reduce((acc, item) => {
    const group = item[key];
    if (!acc[group]) acc[group] = [];
    acc[group].push(item);
    return acc;
  }, {});
}
