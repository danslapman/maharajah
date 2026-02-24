using System;
using System.Collections.Generic;

/// <summary>
/// A min-heap priority queue backed by a list.
/// </summary>
public class MinHeap<T> where T : IComparable<T>
{
    private readonly List<T> _data = new();

    /// <summary>
    /// Inserts an item into the heap, maintaining the heap property.
    /// </summary>
    public void Insert(T item)
    {
        _data.Add(item);
        BubbleUp(_data.Count - 1);
    }

    /// <summary>
    /// Removes and returns the minimum element.
    /// </summary>
    public T ExtractMin()
    {
        if (_data.Count == 0) throw new InvalidOperationException("Heap is empty");
        T min = _data[0];
        _data[0] = _data[^1];
        _data.RemoveAt(_data.Count - 1);
        if (_data.Count > 0) SiftDown(0);
        return min;
    }

    private void BubbleUp(int i)
    {
        while (i > 0)
        {
            int parent = (i - 1) / 2;
            if (_data[parent].CompareTo(_data[i]) <= 0) break;
            (_data[parent], _data[i]) = (_data[i], _data[parent]);
            i = parent;
        }
    }

    private void SiftDown(int i)
    {
        int n = _data.Count;
        while (true)
        {
            int smallest = i, l = 2 * i + 1, r = 2 * i + 2;
            if (l < n && _data[l].CompareTo(_data[smallest]) < 0) smallest = l;
            if (r < n && _data[r].CompareTo(_data[smallest]) < 0) smallest = r;
            if (smallest == i) break;
            (_data[i], _data[smallest]) = (_data[smallest], _data[i]);
            i = smallest;
        }
    }
}

/// <summary>
/// Utility methods for string manipulation.
/// </summary>
public static class StringUtils
{
    /// <summary>
    /// Truncates a string to at most maxLength characters, appending "..." if cut.
    /// </summary>
    public static string Truncate(string s, int maxLength)
    {
        if (s.Length <= maxLength) return s;
        return s[..(maxLength - 3)] + "...";
    }
}
