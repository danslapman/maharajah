import React, { useState } from "react";

/** Props for the Counter component. */
interface CounterProps {
  initialValue?: number;
  step?: number;
}

/**
 * A simple counter component that increments or decrements by a step.
 */
function Counter({ initialValue = 0, step = 1 }: CounterProps) {
  const [count, setCount] = useState(initialValue);
  return (
    <div>
      <button onClick={() => setCount(c => c - step)}>-</button>
      <span>{count}</span>
      <button onClick={() => setCount(c => c + step)}>+</button>
    </div>
  );
}

/**
 * Formats a numeric value as a currency string.
 * @param amount - The numeric amount to format.
 * @param currency - ISO 4217 currency code, defaults to "USD".
 */
function formatCurrency(amount: number, currency = "USD"): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency }).format(amount);
}

/** Represents a labeled item in a list. */
interface ListItem {
  id: number;
  label: string;
}
