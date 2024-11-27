# bot-swbox

## Aperçu
`bot-swbox` est un bot Discord développé en Rust conçu pour améliorer l’expérience des utilisateurs en fournissant diverses commandes permettant d’afficher des statistiques de jeu et des données de classement. Le bot est conçu pour les joueurs de Summoners War.

Pour utiliser le bot, ajoutez-le à votre serveur Discord en contactant B4tiste sur Discord (tag : b4tiste).

## Fonctionnalités
- **Commandes interactives** : Utilisez une série de commandes slash pour accéder aux fonctionnalités du bot.
- **Statistiques de jeu** : Récupérez et affichez les statistiques des monstres pour différentes saisons.
- **Informations sur les classements** : Consultez les classements et leurs détails.
- **Menu d’aide** : Accédez facilement à la liste des commandes disponibles et leurs descriptions.

---

## Roadmap des fonctionnalités

### ToDo :

- [ ] Refaire les commandes de suivi de compte pour plus de clarté et de performance.
- [ ] Introduire une commande pour les statistiques des joueurs sur les trois dernières saisons.
- [ ] Étendre les commandes de statistiques des monstres pour inclure les données des saisons précédentes.

### Terminé :

- [x] Ajouter la commande /help
- [x] Check si il y a un 2A dans la liste des monstres recherchés, si oui, le bot doit le choisir en priorité
- [x] Ajouter le choix de choisir le numéro de saison pour les stats de monstre
- [x] Ajouter une commande pour afficher les taux de victoire communs de deux monstres joués ensemble. Affiche aussi le taux de victoire de l'un contre l'autre.

---

## Guide utilisateur

### `/help`
**Description** : Affiche les commandes disponibles et leurs descriptions.

**Utilisation** :
- Tapez `/help` dans le chat Discord pour afficher la liste de toutes les commandes supportées.

**Résultat** :
- Un message intégré (embed) bien formaté avec :
  - Une liste des commandes avec leurs descriptions.
  - Les détails des créateurs.
  - Un lien vers le code source et la roadmap du projet.

---

### `/get_mob_stats`
**Description** : Récupère les statistiques des monstres, avec une option pour spécifier la saison.

**Utilisation** :
- `/get_mob_stats` => Ouverture d'un formulaire pour saisir le nom du monstre et la saison (optionnel).

**Fonctionnalités** :
- Priorise automatiquement les monstres 2A dans les recherches lorsque cela est applicable.
- Permet de récupérer des données spécifiques à une saison.

---

### `/get_ranks`
**Description** : Affiche les informations détaillées des classements actuels de RTA.

**Utilisation** :
- `/get_ranks`

**Résultat** :
- Présente les données des classements dans un format facile à lire.

---

## Contributions
Ce projet est maintenu et développé par :
- [B4tiste](https://github.com/B4tiste)
- [shvvkz](https://github.com/shvvkz)

Si vous souhaitez contribuer à ce projet, veuillez contacter B4tiste sur Discord (tag : b4tiste)

---
