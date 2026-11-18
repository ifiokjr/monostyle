/// A ticket list screen: Dart interpolation in every form, nested widget trees, and the
/// null-aware operators a scanner must not read as ternaries.
import 'dart:async';

/// A support ticket.
class Ticket {
  const Ticket({required this.id, required this.title, this.assignee, this.priority = 3});

  final int id;
  final String title;
  final String? assignee;
  final int priority;

  String get label => '#$id $title';

  String get owner => assignee ?? 'unassigned';

  @override
  String toString() => 'Ticket(${id.toString().padLeft(4, '0')}, "$title", $owner)';
}

/// Escapes for a shell, in a raw string where `\` and `$` are just characters.
const shellEscape = r'\$\{HOME\}';

/// A multi-line description with interpolation inside it.
String describe(Ticket ticket) {
  final urgency = switch (ticket.priority) {
    1 => 'urgent',
    2 => 'high',
    3 => 'normal',
    _ => 'low',
  };

  return '''
Ticket ${ticket.id}: ${ticket.title}
Owner: ${ticket.assignee?.toUpperCase() ?? "nobody"}
Urgency: $urgency (${ticket.priority > 2 ? 'can wait' : 'act now'})
''';
}

/// Groups tickets by owner, keeping the insertion order of first sight.
Map<String, List<Ticket>> byOwner(Iterable<Ticket> tickets) {
  final groups = <String, List<Ticket>>{};

  for (final ticket in tickets) {
    groups.putIfAbsent(ticket.owner, () => <Ticket>[]).add(ticket);
  }

  return groups;
}

/// Streams tickets as they are triaged, one per tick.
Stream<Ticket> triage(List<Ticket> backlog) async* {
  for (final ticket in backlog) {
    await Future<void>.delayed(const Duration(milliseconds: 10));

    if (ticket.priority == 1) {
      yield ticket;
    } else if (ticket.assignee != null) {
      yield ticket;
    }
  }
}

class TicketList extends StatelessWidget {
  const TicketList({super.key, required this.tickets, this.onTap});

  final List<Ticket> tickets;
  final void Function(Ticket)? onTap;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('${tickets.length} tickets')),
      body: ListView.builder(
        itemCount: tickets.length,
        itemBuilder: (context, index) {
          final ticket = tickets[index];

          return ListTile(
            leading: CircleAvatar(child: Text('${ticket.priority}')),
            title: Text(ticket.label),
            subtitle: Text(ticket.owner),
            trailing: ticket.assignee == null
                ? const Icon(Icons.person_off)
                : Icon(Icons.person, color: ticket.priority < 2 ? Colors.red : null),
            onTap: () => onTap?.call(ticket),
          );
        },
      ),
    );
  }
}
