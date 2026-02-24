# Converts a string from snake_case to CamelCase.
def camelize(str)
  str.split('_').map(&:capitalize).join
end

# Wraps text at word boundaries to at most `width` characters per line.
def word_wrap(text, width: 80)
  text.gsub(/(.{1,#{width}})(\s+|\z)/, "\\1\n").rstrip
end

# Counts the frequency of each word in the given string.
# Returns a hash mapping word to its occurrence count.
def word_frequency(text)
  text.downcase.scan(/\w+/).each_with_object(Hash.new(0)) do |word, counts|
    counts[word] += 1
  end
end

# A simple LRU cache backed by a hash with eviction on overflow.
class LruCache
  def initialize(capacity)
    @capacity = capacity
    @store = {}
  end

  # Retrieves a value by key, or nil if absent.
  def get(key)
    return nil unless @store.key?(key)
    @store[key] = @store.delete(key)
  end

  # Stores a key-value pair, evicting the least recently used entry if full.
  def put(key, value)
    @store.delete(key)
    @store[key] = value
    @store.shift if @store.size > @capacity
  end
end
