/**
 * Represents a 2D vector with x and y components.
 */
data class Vector2(val x: Double, val y: Double) {
    /** Returns the magnitude (length) of this vector. */
    fun magnitude(): Double = Math.sqrt(x * x + y * y)

    /** Returns a new vector scaled by the given factor. */
    fun scale(factor: Double): Vector2 = Vector2(x * factor, y * factor)

    /** Returns the dot product of this vector and another. */
    fun dot(other: Vector2): Double = x * other.x + y * other.y
}

/**
 * Computes the Euclidean distance between two 2D points.
 */
fun distance(x1: Double, y1: Double, x2: Double, y2: Double): Double {
    val dx = x2 - x1
    val dy = y2 - y1
    return Math.sqrt(dx * dx + dy * dy)
}

/**
 * Clamps a value to the closed interval [lo, hi].
 */
fun clamp(value: Double, lo: Double, hi: Double): Double =
    maxOf(lo, minOf(hi, value))

/**
 * A simple 2D axis-aligned bounding box.
 */
class BoundingBox(val minX: Double, val minY: Double, val maxX: Double, val maxY: Double) {

    /**
     * Constructs a bounding box from a center point and half-extents.
     */
    constructor(centerX: Double, centerY: Double, halfW: Double, halfH: Double, dummy: Unit = Unit) :
        this(centerX - halfW, centerY - halfH, centerX + halfW, centerY + halfH)

    // plain implementation note — not KDoc
    fun width(): Double = maxX - minX

    fun height(): Double = maxY - minY

    /** Returns true if the given point is inside or on the boundary. */
    fun contains(px: Double, py: Double): Boolean =
        px in minX..maxX && py in minY..maxY
}

/**
 * Utility functions for angle conversion.
 */
object Angles {
    /** Converts degrees to radians. */
    fun toRadians(degrees: Double): Double = degrees * Math.PI / 180.0

    /** Converts radians to degrees. */
    fun toDegrees(radians: Double): Double = radians * 180.0 / Math.PI
}

/** A type alias for a list of 2D vectors. */
typealias Path2D = List<Vector2>
