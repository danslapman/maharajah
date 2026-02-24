/**
 * Represents a 2D point with x and y coordinates.
 */
public class Point {
    private final double x;
    private final double y;

    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }

    /**
     * Computes the Euclidean distance to another point.
     */
    public double distanceTo(Point other) {
        double dx = this.x - other.x;
        double dy = this.y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }

    /**
     * Returns a new point translated by the given offsets.
     */
    public Point translate(double dx, double dy) {
        return new Point(x + dx, y + dy);
    }

    @Override
    public String toString() {
        return "(" + x + ", " + y + ")";
    }
}

/**
 * Computes the area of a circle given its radius.
 */
public static double circleArea(double radius) {
    return Math.PI * radius * radius;
}

/**
 * Clamps a value to the range [min, max].
 */
public static double clamp(double value, double min, double max) {
    return Math.max(min, Math.min(max, value));
}
