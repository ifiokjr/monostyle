// String templates including the ${'$'} escape form.

class User(val name: String)

fun shout(user: User?): String {
    val a = "${user?.name ?: "anon"} says ${'$'}hi"
    val b = "braces {x} and ${user?.name?.length ?: 0} chars"

    return a + b
}

fun main() {
    val u = User("ada")
    val s = shout(u)

    println(s)
}
