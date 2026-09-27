import java.util.List;
import java.util.Map;

@Service
public class Registry<T extends Comparable<T>> {
    private final Map<String, List<T>> entries = new HashMap<>();



    public Optional<T> find(String key) {
        return entries.getOrDefault(key, List.of()).stream().findFirst();
    }
}
