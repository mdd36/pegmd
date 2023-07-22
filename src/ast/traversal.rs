use super::model::Node;

///
pub enum Direction {
  Entering,
  Exiting,
}

/// 
pub enum NextAction {
  GotoNext,
  SkipChildren,
  End
}


/// 
pub trait Visitor {
  fn visit(&self, node: &Node, action: Direction) -> NextAction;
}

impl <'a> Node<'a> {

  /// Walk over the tree, starting at this node and continuing recursively until either all nodes are
  /// visited or the visitor signals to stop the traversal. 
  /// 
  /// All nodes with children are visited twice, once before visiting their children
  /// and once after visiting their children. The visitor can determine whether this is an entry or exist
  /// visit using the [Direction] variant provided to it. No additional nodes will be visited after receiving
  /// [`NextAction::End`] from the visitor, but the traversal will complete exit visits to any nodes with 
  /// children that haven't already received one. Nodes without children are only visited once.
  /// 
  /// ### Parameters
  /// 
  /// - `visitor`: Any implementor of the [`Visitor`] trait
  /// 
  /// ### Returns
  /// 
  /// Pipes through the visitor's NextAction value to be used in the recursive call. Any expected output from the
  /// recursion should be generated by side effects in the visitor as it visits each node.
  pub fn traverse(&self, visitor: &impl Visitor) -> NextAction {
      let children = match self.children() {
          Some(c) => c,
          None => return visitor.visit(self, Direction::Entering), 
      };
  
      match visitor.visit(self, Direction::Entering) {
          NextAction::GotoNext => {
              // Visit the children, stopping early if one of them says to end the traversal
              for child in children.iter() {
                  if let NextAction::End = child.traverse(visitor) {
                      return NextAction::End
                  }
              }
              visitor.visit(self, Direction::Exiting)
          }
          NextAction::SkipChildren => {
              // Give the container its exit visit since we're not visiting any children
              visitor.visit(self, Direction::Exiting)
          }
          NextAction::End => {
              // Give the container its exit visit before stopping the traversal
              let _ = visitor.visit(self, Direction::Exiting);
              NextAction::End
          }
      }
  }
}