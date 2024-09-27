from discord.ext import commands
from lib import infoRankSW, infoPlayerSwarena, infoMobSwarena
import requests

url_reddit_ranks = "https://www.reddit.com/r/summonerswar/comments/1fjf8zj/rta_season_30_cutoff_megathread/"

# Création d'un cog pour les commandes
class CommandsCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    # Commande d'aide
    @commands.command()
    async def help(self, ctx):
        message = (
            "```"  # Bloc de code Markdown
            "Commandes disponibles:\n"
            "!ranks: Récupérer les scores des rangs actuels\n"
            "!mobstats <monstre>: Récupérer les stats d'un monstre\n"
            "WIP !histopseudo <pseudo>: Récupérer les saisons d'un joueur via son id\n"
            "WIP !histoid <id>: Récupérer les saisons d'un joueur via son pseudo\n"
            "WIP !playerStat <pseudo>: Récupérer les stats récentes d'un joueur\n"
            "-------Admin-------\n"
            "!reload <cog>: Recharger un cog dynamiquement (commands_cog)\n"
            "```"
        )
        await ctx.send(f"{ctx.author.mention}\n{message}\n Bot créé par <@191619427584835585>")

    # Commande pour récupérer les scores des rangs
    @commands.command()
    async def ranks(self, ctx):
        scores = infoRankSW()
        if scores:
            message = (
                "```"
                f"Ranks   | Scores actuels\n"
                f"--------|-------\n"
                f"C1      | {scores['c1']}\n"
                f"C2      | {scores['c2']}\n"
                f"C3      | {scores['c3']}\n"
                f"P1      | {scores['p1']}\n"
                f"P2      | {scores['p2']}\n"
                f"P3      | {scores['p3']}\n"
                f"G1      | {scores['g1']}\n"
                f"G2      | {scores['g2']}\n"
                f"G3      | {scores['g3']}\n"
                "```"
                f"Prediction Cutoff Reddit : {url_reddit_ranks}\n"
            )
            await ctx.send(f"{ctx.author.mention}\n{message}")
        else:
            await ctx.send("Erreur lors de la récupération des données.")

    # Commande pour récupérer les saisons d'un joueur via son id ou son pseudo
    @commands.command()
    async def PtrackSwarena(self, ctx, type: str, player):
        if type == "id":
            player_data = infoPlayerSwarena(player)
        elif type == "pseudo":
            url = "https://api.swarena.gg/player/search/" + player
            response = requests.get(url)
            data = response.json()
            if "data" in data and data["data"]:
                player_id = data["data"][0]["id"]
                player_data = infoPlayerSwarena(player_id)
            else:
                player_data = None

        if player_data:
            for season in player_data:
                await ctx.send(f"__Saison__ {season['season']}:\nNom: {season['name']}\nRank: {season['rank']}\nPays: {season['country']}\nPhoto URL: {season['picture']}")
        else:
            await ctx.send("Aucune saison disponible pour ce joueur.")

    # Commande pour obtenir les infos d'un monstre
    @commands.command()
    async def mobstats(self, ctx, *, mob: str):  # Utiliser * pour capturer l'argument entier, même avec des espaces

        # Récupérer le slug
        slug_url = f"https://api.swarena.gg/monster/search/{mob}"
        response = requests.get(slug_url)
        data = response.json()

        if "data" in data and data["data"]:
            mob_formatted = data["data"][0]["slug"]
            mob_name = data["data"][0]["name"]
        else:
            await ctx.send("Apprends à écrire, ce monstre n'existe pas.")
            return

        # Récupération de l'id du monstre
        url = f"https://api.swarena.gg/monster/{mob_formatted}/details"
        response = requests.get(url)
        data = response.json()

        if "data" in data and data["data"]:
            mob_id = data["data"]["id"]
        else:
            await ctx.send("Erreur lors de la récupération des données.")
            return

        # Récupération des stats du monstre
        mob_data = infoMobSwarena(mob_id)

        message = (
            "```"  # Bloc de code Markdown
            f"Stats du monstre {mob_name}:\n"
            f"--------Hors G3--------\n"
            f"Play rate: {mob_data['no g3']['data']['play_rate']}% ({mob_data['no g3']['data']['played']})\n"
            f"Win rate: {mob_data['no g3']['data']['win_rate']}% ({mob_data['no g3']['data']['winner']})\n"
            f"Ban rate: {mob_data['no g3']['data']['ban_rate']}% ({mob_data['no g3']['data']['banned']})\n"
            f"Lead rate: {mob_data['no g3']['data']['lead_rate']}% ({mob_data['no g3']['data']['leader']})\n"
            f"----------G3----------\n"
            f"Play rate: {mob_data['g3']['data']['play_rate']}% ({mob_data['g3']['data']['played']})\n"
            f"Win rate: {mob_data['g3']['data']['win_rate']}% ({mob_data['g3']['data']['winner']})\n"
            f"Ban rate: {mob_data['g3']['data']['ban_rate']}% ({mob_data['g3']['data']['banned']})\n"
            f"Lead rate: {mob_data['g3']['data']['lead_rate']}% ({mob_data['g3']['data']['leader']})\n"
            "```"
        )

        await ctx.send(f"{ctx.author.mention}\n{message}")


# Fonction nécessaire pour charger le cog
async def setup(bot):
    await bot.add_cog(CommandsCog(bot))
