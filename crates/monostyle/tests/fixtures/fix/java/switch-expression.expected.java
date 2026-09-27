public class Labels {
    public static String name(int code) {
        return switch (code) {
            case 1 -> "one";
            case 2, 3 -> "few";
            default -> {
                yield "many {kept}";
            }
        };
    }
}
