import kotlin.properties.Delegates

class Watcher {
    var value: String by Delegates.observable("<empty>") { _, old, new ->
        println("$old -> $new")
    }

    val computed: Int by lazy {
        42
    }
}
