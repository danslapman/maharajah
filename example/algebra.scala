/** A simple immutable rational number in lowest terms. */
case class Rational(numer: Int, denom: Int) {
  require(denom != 0, "denominator cannot be zero")
  private val g = gcd(numer.abs, denom.abs)
  val n: Int = numer / g
  val d: Int = denom / g

  /** Adds two rational numbers. */
  def +(other: Rational): Rational = Rational(n * other.d + other.n * d, d * other.d)

  /** Multiplies two rational numbers. */
  def *(other: Rational): Rational = Rational(n * other.n, d * other.d)

  override def toString: String = s"$n/$d"
}

object Rational {
  /** Returns the greatest common divisor of two non-negative integers. */
  def gcd(a: Int, b: Int): Int = if (b == 0) a else gcd(b, a % b)
}

/** Checks whether an integer is a perfect square. */
def isPerfectSquare(n: Int): Boolean =
  val s = math.sqrt(n.toDouble).toInt
  s * s == n

/** Flattens a nested list one level deep. */
def flatten[A](xss: List[List[A]]): List[A] = xss.flatMap(identity)
