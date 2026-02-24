module Numerics

/// Computes integer power b^e using fast exponentiation.
let rec pow (b: int) (e: int) : int =
    if e = 0 then 1
    elif e % 2 = 0 then
        let half = pow b (e / 2)
        half * half
    else
        b * pow b (e - 1)

/// Returns the greatest common divisor of two integers using Euclid's algorithm.
let rec gcd (a: int) (b: int) : int =
    if b = 0 then abs a else gcd b (a % b)

/// Checks whether n is prime using trial division.
let isPrime (n: int) : bool =
    if n < 2 then false
    else Seq.forall (fun d -> n % d <> 0) (seq { 2 .. int (sqrt (float n)) })

/// Converts a decimal integer to its binary string representation.
let toBinary (n: int) : string =
    if n = 0 then "0"
    else
        let rec go acc m =
            if m = 0 then acc
            else go (string (m % 2) + acc) (m / 2)
        go "" n
