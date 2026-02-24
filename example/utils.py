def parse_args(argv):
    """Parse command-line arguments into a dict.

    Expects flags of the form --key value or --flag (boolean).
    Returns a dictionary mapping flag names to their values.
    """
    result = {}
    it = iter(argv)
    for token in it:
        if token.startswith("--"):
            key = token[2:]
            try:
                val = next(it)
                result[key] = val
            except StopIteration:
                result[key] = True
    return result


def chunk_list(lst, size):
    """Split a list into chunks of at most `size` elements.

    >>> list(chunk_list([1,2,3,4,5], 2))
    [[1, 2], [3, 4], [5]]
    """
    for i in range(0, len(lst), size):
        yield lst[i:i + size]


class RingBuffer:
    """A fixed-size circular buffer that overwrites the oldest entry when full."""

    def __init__(self, capacity):
        self._buf = [None] * capacity
        self._head = 0
        self._size = 0

    def push(self, item):
        """Append an item, evicting the oldest if the buffer is full."""
        self._buf[self._head % len(self._buf)] = item
        self._head += 1
        self._size = min(self._size + 1, len(self._buf))

    def to_list(self):
        """Return the buffer contents in insertion order."""
        if self._size < len(self._buf):
            return self._buf[:self._size]
        start = self._head % len(self._buf)
        return self._buf[start:] + self._buf[:start]
