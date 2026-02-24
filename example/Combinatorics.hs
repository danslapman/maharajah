module Combinatorics where

-- | Computes the binomial coefficient "n choose k".
choose :: Integer -> Integer -> Integer
choose n k
  | k < 0 || k > n = 0
  | k == 0 || k == n = 1
  | otherwise = choose (n-1) (k-1) + choose (n-1) k

-- | Returns the nth Fibonacci number using fast doubling.
fibonacci :: Integer -> Integer
fibonacci n = go n
  where
    go 0 = 0
    go 1 = 1
    go m
      | even m    = let h = go (m `div` 2)
                        k = go (m `div` 2 + 1)
                    in h * (2 * k - h)
      | otherwise = let h = go ((m - 1) `div` 2)
                        k = go ((m + 1) `div` 2)
                    in h * h + k * k

-- | Removes consecutive duplicate elements from a list.
compress :: Eq a => [a] -> [a]
compress [] = []
compress [x] = [x]
compress (x:y:rest)
  | x == y    = compress (y : rest)
  | otherwise = x : compress (y : rest)
