// Trailing lambda syntax on collection functions.

data class Item(val name: String)

fun names(items: List<Item>): List<String> {
    val result = items.map { it.name }.filter { it.startsWith("a") }

    return result
}

fun main() {
    val items = listOf(Item("axe"), Item("bow"))
    val out = names(items)

    println(out.joinToString { "{it}" })
}
