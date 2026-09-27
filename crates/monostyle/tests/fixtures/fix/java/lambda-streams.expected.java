import java.util.List;

public class Report {
    public String render(List<Integer> values) {
        var total = values.stream().mapToInt(Integer::intValue).sum();
        var label = values.stream()
            .filter(v -> v > 0)
            .map(v -> "item {kept} " + v)
            .reduce("", (a, b) -> a + b);

        return label + total;
    }
}
