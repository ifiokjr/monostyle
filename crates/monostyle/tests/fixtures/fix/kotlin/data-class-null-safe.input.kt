// Data class with null-safe chains and elvis, some inside strings.

data class Person(val name: String, val age: Int?)

fun label(p: Person?): String {
    val text = p?.let { "${it.name} is ${it.age ?: 0}" } ?: "empty {slot}"
    return text
}

fun main() {
    val p: Person? = Person("kim", null)
    val out = label(p)




    println(out)
}
